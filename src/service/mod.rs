use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;

#[derive(Debug)]
pub enum ProviderError {
    ServiceNotFound,
    MethodNotFound,
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    InternalError(Box<dyn std::error::Error + Sync + Send>),
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

pub struct Request<S = ()> {
    pub service: String,
    pub method: String,
    pub session: Arc<S>,
}

pub type Response<E> = Result<Bytes, E>;

impl ProviderError {
    pub fn to_bytes(&self) -> Bytes {
        match self {
            Self::ServiceNotFound => "ServiceNotFound",
            Self::MethodNotFound => "MethodNotFound",
            // FIXME This should probably be a in internal server error instead?
            Self::SerializerError(_) => "SerializerError",
            Self::DeserializerError(_) => "DeserializerError",
            Self::InternalError(_) => "InternalError",
        }
        .into()
    }
}

#[async_trait]
pub trait Consumer: Sync + Send {
    //fn name(&self) -> &'static str;
    async fn call(&self, service: &str, method: &str, data: Bytes) -> Result<Bytes, ConsumerError>;
}

#[async_trait]
pub trait Provider<S: Sync + Send>: Sync + Send {
    fn name(&self) -> &'static str;
    async fn call(&self, request: &Request<S>, data: Bytes) -> Result<Bytes, ProviderError>;
}

pub struct ProviderRegistry<S: Sync + Send>
where
    Self: Sync + Send,
{
    providers: DashMap<String, Arc<dyn Provider<S>>>,
}

impl<S: Sync + Send> ProviderRegistry<S> {
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
        }
    }
    pub fn register<P: Provider<S> + 'static>(&mut self, provider: P) {
        self.providers
            .insert(provider.name().to_owned(), Arc::new(provider));
    }
    pub fn get(&self, service_name: &str) -> Option<Arc<dyn Provider<S>>> {
        self.providers.get(service_name).map(|arc| arc.clone())
    }
}
