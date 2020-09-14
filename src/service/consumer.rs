//! Service consumer
//!

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::oneshot;
use bytes::Bytes;

/// This response object implements a future that can be polled in order
/// to wait for the result of a method. It does however not need to be
/// polled to make progress and shouldn't be polled if it was created as
/// part of a notification to multiple recipients (broadcast).
#[must_use = "The response should either be polled or `assert_broadcast` should be called on it (consuming it)"]
pub struct Response {
    rx: Option<oneshot::Receiver<Result<Bytes, ConsumerError>>>,
}

impl Response {
    /// Create a new response object from a oneshot receiver
    pub fn new(rx: oneshot::Receiver<Result<Bytes, ConsumerError>>) -> Self {
        Self {
            rx: Some(rx)
        }
    }
    /// Create a broadcast response object which always resolves
    /// to `Err(ConsumerError::Broadcast)` when polled.
    pub fn notification() -> Self  {
        Self {
            rx: None
        }
    }
    /// Assert that the response object was returned by a notification
    /// call. This is equal t ocalling `drop()` but also fails with an
    /// error if the response object was created by a call to request.
    pub fn assert_notification(self) {
        assert!(self.rx.is_none())
    }
}

impl Future for Response {
    type Output = Result<Bytes, ConsumerError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(rx) = self.rx.as_mut() {
            match Pin::new(rx).poll(cx) {
                Poll::Ready(Ok(response)) => Poll::Ready(response),
                Poll::Ready(Err(_)) => Poll::Ready(Err(ConsumerError::Disconnected)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Err(ConsumerError::Broadcast))
        }
    }
}


/// This trait is implemented by all generic consumers and used by the
/// generated service code.
pub trait Consumer: Sync + Send {
    /// Call service method
    fn request(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> Response;
    /// Notify service method
    fn notify(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    );
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
