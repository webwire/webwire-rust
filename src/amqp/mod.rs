//! This module is work in progress and mostly undocumented
// FIXME remove this when the documentation is complete
#![allow(missing_docs)]

use std::{collections::HashSet, sync::Arc};

use bytes::Bytes;
use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicRejectOptions,
        ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, Channel,
};

use crate::{Consumer, ConsumerError, NamedProvider, Provider, ProviderError, Router};

mod config;
pub use config::AMQPConfig;

pub struct AMQPProvider {
    config: AMQPConfig,
    router: Router<()>,
    service_names: HashSet<String>,
}

async fn declare(channel: &Channel, config: &AMQPConfig) -> Result<(), lapin::Error> {
    channel
        .exchange_declare(
            &config.exchange_name,
            lapin::ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    channel
        .queue_declare(
            &config.queue_name,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    Ok(())
}

impl AMQPProvider {
    pub fn new(config: AMQPConfig) -> Self {
        Self {
            config,
            router: Router::new(),
            service_names: HashSet::new(),
        }
    }
    pub fn service<P>(&mut self, provider: P)
    where
        P: NamedProvider<()> + 'static,
    {
        self.router.service(provider);
        self.service_names.insert(P::NAME.to_owned());
    }
    pub async fn run(&self) -> Result<(), lapin::Error> {
        let connection = self.config.connect().await?;
        let channel = connection.create_channel().await?;

        declare(&channel, &self.config);

        for service_name in self.service_names.iter() {
            channel
                .queue_bind(
                    &self.config.queue_name,
                    &self.config.exchange_name,
                    &format!("{}.*", service_name),
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
        }

        let mut consumer = channel
            .basic_consume(
                &self.config.queue_name,
                "my_consumer", // XXX
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok((channel, delivery)) => {
                    let parts = delivery
                        .routing_key
                        .as_str()
                        .rsplitn(2, '.')
                        .collect::<Vec<_>>();
                    if parts.len() != 2 {
                        println!("Unsupported routing_key: {}", delivery.routing_key);
                        delivery.reject(BasicRejectOptions::default()).await?;
                        continue;
                    }
                    let service_name = parts[1];
                    let method_name = parts[0];
                    // FIXME why does the data need to be cloned here?
                    // There must be a better way...
                    let data = Bytes::from(delivery.data.clone());
                    let response = self
                        .router
                        .call(&Arc::new(()), service_name, method_name, data)
                        .await;
                    match response {
                        Ok(data) => {
                            // FIXME what to do with this?
                            delivery.ack(BasicAckOptions::default()).await?;
                        }
                        Err(e) => {
                            // FIXME reject because of unknown method
                            println!("{:?}", e);
                            delivery.reject(BasicRejectOptions::default()).await?;
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}

pub struct AMQPConsumer {
    config: AMQPConfig,
}

impl AMQPConsumer {
    pub fn new(config: AMQPConfig) -> Self {
        Self { config }
    }
}

impl Consumer for AMQPConsumer {
    fn request(
        &self,
        service: &str,
        method: &str,
        data: bytes::Bytes,
    ) -> crate::service::consumer::Response {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<bytes::Bytes, ConsumerError>>();
        let config = self.config.clone();
        let service = service.to_owned();
        let method = method.to_owned();
        tokio::spawn(async move {
            let connection = config.connect().await.unwrap();
            let channel = connection.create_channel().await.unwrap();
            declare(&channel, &config).await.unwrap();
            let result = channel
                .basic_publish(
                    &config.exchange_name,
                    &format!("{}.{}", service, method),
                    BasicPublishOptions::default(),
                    data.to_vec(),
                    BasicProperties::default(),
                )
                .await;
            match result {
                Ok(confirm) => {
                    match confirm.await {
                        Ok(confirmation) => {
                            // FIXME this is wrong
                            let response = Bytes::new();
                            tx.send(Ok(response))
                        }
                        Err(e) => tx.send(Err(ConsumerError::Transport(Box::new(e)))),
                    }
                }
                Err(e) => tx.send(Err(ConsumerError::Transport(Box::new(e)))),
            }
        });
        crate::service::consumer::Response::new(rx)
    }
    fn notify(&self, service: &str, method: &str, data: bytes::Bytes) {
        todo!()
    }
}
