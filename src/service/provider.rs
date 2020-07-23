//! Service provider

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use futures::future::{ready, BoxFuture};

/// Service provider
pub trait Provider<S: Sync + Send>: Sync + Send {
    //fn name(&self) -> &'static str;
    /// Call a method of this service
    fn call(
        &self,
        session: &Arc<S>,
        method: &str,
        data: Bytes,
    ) -> BoxFuture<Result<Bytes, ProviderError>>;
}

/// This trait adds a service name to the service provider
/// which is used for the router.
pub trait NamedProvider<S: Sync + Send>: Provider<S> {
    /// Name of the provided service
    const NAME: &'static str;
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

/// The router is used to register service provider and dispatch
/// requests.
pub struct Router<S: Sync + Send> {
    services: HashMap<String, Box<dyn Provider<S>>>,
}

impl<S: Sync + Send> Router<S> {
    /// Create an empty router
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
    /// Add service provider to this router
    pub fn service<P>(&mut self, provider: P)
    where
        P: NamedProvider<S> + 'static,
    {
        self.services.insert(P::NAME.to_owned(), Box::new(provider));
    }
    /// Call a service
    pub fn call(
        &self,
        session: &Arc<S>,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> BoxFuture<Result<Bytes, ProviderError>> {
        match self.services.get(service) {
            Some(provider) => provider.call(session, method, data),
            None => Box::pin(ready(Err(ProviderError::ServiceNotFound))),
        }
    }
}
