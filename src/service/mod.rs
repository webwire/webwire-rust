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
    pub session: S,
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
pub trait Consumer {
    async fn call(&self, request: &Request, data: Bytes) -> Result<Bytes, ConsumerError>;
}

#[async_trait]
pub trait Provider<S = ()>: Sync + Send {
    async fn call(&self, request: &Request<S>, data: Bytes) -> Result<Bytes, ProviderError>;
}

pub trait ServiceFactory<S: Sync + Send>
where
    Self: Sync + Send,
{
    fn name(&self) -> &'static str;
    fn create(&self, session: &Arc<S>) -> Box<dyn Provider>;
}

pub struct ServiceRegistry<S: Sync + Send> {
    inner: Arc<ServiceRegistryInner<S>>,
}

impl<S: Sync + Send> Clone for ServiceRegistry<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S: Sync + Send> ServiceRegistry<S> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ServiceRegistryInner {
                services: DashMap::new(),
            }),
        }
    }
    pub fn register<F: ServiceFactory<S> + 'static>(&mut self, factory: F) {
        self.inner
            .services
            .insert(factory.name().to_owned(), Box::new(factory));
    }
    pub fn get(&self, service_name: &str, session: &Arc<S>) -> Option<Box<dyn Provider>> {
        self.inner
            .services
            .get(service_name)
            .map(|factory| factory.create(session))
    }
}

pub struct ServiceRegistryInner<S: Sync + Send> {
    services: DashMap<String, Box<dyn ServiceFactory<S>>>,
}
