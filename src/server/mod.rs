use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;

use crate::{Consumer, ConsumerError, Provider, ProviderError, Request};

pub mod hyper;

#[derive(Clone)]
pub struct Server {
    inner: Arc<ServerInner>,
}

struct ServerInner {
    connections: Vec<ServerConnection>,
    providers: DashMap<String, Box<dyn Provider + Sync + Send>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ServerInner {
                connections: Vec::new(),
                providers: DashMap::new(),
            }),
        }
    }
    pub fn provider<P: Provider + Sync + Send + 'static>(&mut self, provider: P) {
        self.inner
            .providers
            .insert(provider.name().to_owned(), Box::new(provider));
    }
    pub fn connections(&self) -> std::slice::Iter<ServerConnection> {
        self.inner.connections.iter()
    }
    pub async fn call(&self, method: &str, data: Bytes) -> Result<Vec<u8>, ProviderError> {
        // FIXME add namespace/service support
        let parts = method.split(".").collect::<Vec<_>>();
        if parts.len() != 2 {
            // FIXME actually we should return some kind of InvalidRequest showing that the
            // method name is malformed
            return Err(ProviderError::ServiceNotFound);
        }
        let service_name: &str = parts.get(0).unwrap();
        let method_name: &str = parts.get(1).unwrap();
        let provider = self
            .inner
            .providers
            .get(service_name)
            .ok_or(ProviderError::ServiceNotFound)?;
        provider
            .call(&Request {
                service: service_name.to_owned(),
                method: method_name.to_owned(),
                data: data.to_vec(),
            })
            .await
    }
}

#[derive(Clone)]
pub struct ServerConnection {
    // FIXME
}

#[async_trait]
impl Consumer for ServerConnection {
    async fn call(&self, input: &Request) -> Result<Vec<u8>, ConsumerError> {
        // FIXME implement
        unimplemented!();
    }
    fn notify(&self, input: &Request) -> Result<(), ConsumerError> {
        // FIXME implement
        unimplemented!();
    }
}
