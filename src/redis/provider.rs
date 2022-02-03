use std::sync::Arc;

use bytes::Bytes;
use redis::AsyncCommands;

use crate::{rpc::message::Message, Provider, Router};

pub struct RedisProvider {
    client: redis::Client,
    key: String,
    router: Arc<Router<()>>,
}

impl RedisProvider {
    pub fn new(
        connection_info: impl redis::IntoConnectionInfo,
        key: &str,
        router: Router<()>,
    ) -> Result<Self, redis::RedisError> {
        Ok(Self {
            client: redis::Client::open(connection_info)?,
            key: key.to_owned(),
            router: Arc::new(router),
        })
    }
    pub async fn run(self) {
        let mut con = self
            .client
            .get_multiplexed_async_connection()
            .await
            .unwrap();
        let key = &self.key;
        loop {
            let (_, data): (String, Bytes) = con.brpop(key, 0).await.unwrap();
            let message = match Message::parse(&data) {
                Some(message) => message,
                None => {
                    tracing::warn!("Could not parse message: {:?}", data);
                    continue;
                }
            };
            let request = match message {
                Message::Request(msg) => msg,
                msg => {
                    tracing::warn!("Ignoring unsupported message type: {:?}", msg);
                    continue;
                }
            };
            let router = self.router.clone();
            tokio::spawn(async move {
                let fake_session = Arc::new(());
                let response = router
                    .call(
                        &fake_session,
                        &request.service,
                        &request.method,
                        request.data,
                    )
                    .await;
                if let Err(error) = response {
                    tracing::error!(
                        "Method call of {}.{} failed: {:?}",
                        request.service,
                        request.method,
                        error
                    );
                }
            });
        }
    }
}
