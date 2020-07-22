//! Service provider

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;

/// The generic provider trait implemented by the generated code.
#[async_trait]
pub trait Provider<S: Sync + Send>: Sync + Send {
    /// Returns the name of the service
    fn name(&self) -> &'static str;
    /// Call a method of this service
    async fn call(&self, request: &Request<S>, data: Bytes) -> Result<Bytes, ProviderError>;
}

/// The request object used to call providers.
pub struct Request<S> {
    /// Service name
    pub service: String,
    /// Method name
    pub method: String,
    /// Session
    pub session: Arc<S>,
}

/// The response type used when calling providers.
pub type Response<T> = Result<T, ProviderError>;

/// Error returned by the `Provider::call` method
#[derive(Debug)]
pub enum ProviderError {
    /// The requested service was not found.
    ServiceNotFound,
    /// The requested method was not found.
    MethodNotFound,
    /// An error occured while deserializing the request.
    DeserializerError(serde_json::Error),
    /// An error occured while serializing the response.
    SerializerError(serde_json::Error),
    /// Something else went wrong.
    InternalError(Box<dyn std::error::Error + Sync + Send>),
}

impl ProviderError {
    /// Convert error to Bytes
    pub fn to_bytes(&self) -> Bytes {
        match self {
            Self::ServiceNotFound => "ServiceNotFound",
            Self::MethodNotFound => "MethodNotFound",
            // SerializerError maps to InternalError as this is nothing
            // the consumer can do about.
            Self::SerializerError(_) => "InternalError",
            Self::DeserializerError(_) => "DeserializerError",
            Self::InternalError(_) => "InternalError",
        }
        .into()
    }
}

/// Registry for providers
pub struct ProviderRegistry<S: Sync + Send>
where
    Self: Sync + Send,
{
    providers: DashMap<String, Arc<dyn Provider<S>>>,
}

impl<S: Sync + Send> ProviderRegistry<S> {
    /// Create new provider registry
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
        }
    }
    /// Register a provider
    pub fn register<P: Provider<S> + 'static>(&mut self, provider: P) {
        self.providers
            .insert(provider.name().to_owned(), Arc::new(provider));
    }
    /// Get provider by name
    pub fn get(&self, service_name: &str) -> Option<Arc<dyn Provider<S>>> {
        self.providers.get(service_name).map(|arc| arc.clone())
    }
}
