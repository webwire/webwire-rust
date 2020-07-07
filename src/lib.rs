use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;


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
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    ProviderError(ProviderError), // FIXME
    /// The request was sent to multiple recipients therefore fetching a
    /// result is not supported.
    Broadcast,
    /// The remote side disconnected while waiting for a response
    Disconnected,
}

impl From<ProviderError> for ConsumerError {
    fn from(e: ProviderError) -> Self {
        Self::ProviderError(e)
    }
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
