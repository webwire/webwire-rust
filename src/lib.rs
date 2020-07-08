use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;

pub mod rpc;


#[derive(Debug)]
pub enum ProviderError {
    ServiceNotFound,
    MethodNotFound,
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    InternalError(Box<dyn std::error::Error>),
}

#[derive(Debug)]
pub enum ConsumerError {
    ServiceNotFound,
    MethodNotFound,
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    ProviderError,
    /// The data part of the message could not be understood.
    InvalidData(Bytes),
    /// The request was sent to multiple recipients therefore fetching a
    /// result is not supported.
    Broadcast,
    /// The remote side disconnected while waiting for a response
    Disconnected,
}

pub struct Request {
    pub service: String,
    pub method: String,
    pub data: Vec<u8>,
}

pub type Response<E> = Result<Vec<u8>, E>;

#[derive(Clone)]
pub struct ServerConnection {
    // FIXME
}

impl ProviderError {
    fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::ServiceNotFound => "ServiceNotFound",
            Self::MethodNotFound => "MethodNotFound",
            // FIXME This should probably be a in internal server error instead?
            Self::SerializerError(_) => "SerializerError",
            Self::DeserializerError(_) => "DeserializerError",
            Self::InternalError(_) => "InternalError",
        }.into()
    }
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

#[derive(Clone, Default)]
pub struct Server {
    inner: Arc<ServerInner>,
}

#[derive(Default)]
struct ServerInner {
    connections: Vec<ServerConnection>,
    providers: DashMap<String, Box<dyn Provider + Sync + Send>>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
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
        let provider = self.inner.providers.get(service_name).ok_or(ProviderError::ServiceNotFound)?;
        provider.call(&Request {
            service: service_name.to_owned(),
            method: method_name.to_owned(),
            data: data.to_vec(),
        }).await
    }
}

#[async_trait]
pub trait Consumer {
    async fn call(&self, request: &Request) -> Result<Vec<u8>, ConsumerError>;
    fn notify(&self, request: &Request) -> Result<(), ConsumerError>;
}

#[async_trait]
pub trait Provider {
    fn name(&self) -> &'static str;
    async fn call(&self, request: &Request) -> Result<Vec<u8>, ProviderError>;
    async fn notify(&self, request: &Request) -> Result<(), ProviderError>;
}
