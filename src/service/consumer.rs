//! Service consumer

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;

/// This trait is implemented by all generic consumers and used by the
/// generated service code.
#[async_trait]
pub trait Consumer: Sync + Send {
    /// Call service method
    async fn call(
        &self,
        method: &str,
        data: Bytes,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, ConsumerError>> + Send>>;
}

/// This trait adds a service name to the service consumer.
pub trait NamedProvider: Consumer {
    /// Name of the consumed service
    const NAME: &'static str;
}

/// Error returned by the `Consumer::call` method
#[derive(Debug)]
pub enum ConsumerError {
    /// The requested service was not found.
    ServiceNotFound,
    /// The requested method was not found.
    MethodNotFound,
    /// An error occured while serializing the request.
    SerializerError(serde_json::Error),
    /// An error occured while deserializing the response.
    DeserializerError(serde_json::Error),
    /// The provider reported an internal error.
    ProviderError,
    /// The data part of the message could not be understood.
    InvalidData(Bytes),
    /// The request was sent to multiple recipients therefore fetching a
    /// result is not supported.
    Broadcast,
    /// The remote side disconnected while waiting for a response
    Disconnected,
}
