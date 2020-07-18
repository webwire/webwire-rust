use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};

use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use tokio::sync::oneshot;

use crate::service::{ConsumerError, Provider, ProviderError};
use crate::utils::AtomicWeak;

use super::message::{ErrorKind, Message};
use super::transport::{FrameError, FrameHandler, Transport};

pub struct Response {
    is_broadcast: bool,
    result_rx: oneshot::Receiver<Result<Bytes, ConsumerError>>,
}

impl Future for Response {
    type Output = Result<Bytes, ConsumerError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.is_broadcast {
            Poll::Ready(Err(ConsumerError::Broadcast))
        } else {
            match Pin::new(&mut self.result_rx).poll(cx) {
                Poll::Ready(Ok(response)) => Poll::Ready(response),
                Poll::Ready(Err(_)) => Poll::Ready(Err(ConsumerError::Disconnected)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

pub struct Engine
where
    Self: Sync + Send,
{
    listener: AtomicWeak<dyn EngineListener + Sync + Send>,
    transport: Box<dyn Transport + Sync + Send>,
    last_message_id: AtomicU64,
    requests: DashMap<u64, oneshot::Sender<Result<Bytes, ConsumerError>>>,
}

fn _get_data(bytes: &Bytes, slice: Option<&[u8]>) -> Bytes {
    match slice {
        Some(slice) => bytes.slice((slice.as_ptr() as usize - bytes.as_ptr() as usize)..),
        None => Bytes::new(),
    }
}

impl Engine {
    pub fn new<T: Transport + Sync + Send + 'static>(transport: T) -> Self {
        Engine {
            listener: AtomicWeak::default(),
            transport: Box::new(transport),
            last_message_id: AtomicU64::new(0),
            requests: DashMap::new(),
        }
    }

    pub fn start(self: &Arc<Self>, listener: Weak<dyn EngineListener + Sync + Send>) {
        self.listener.replace(listener);
        self.transport.start(Box::new(Arc::downgrade(self)));
    }

    fn send(&self, frame: Bytes) {
        // If sending fails this means that the sender task has stopped and
        // thus dropped the receiving end of the channel. This is expected
        // behavior when the engine and transport are being terminated and
        // there are still some requests being processed.
        let _ = self.transport.send(frame);
    }

    fn next_message_id(&self) -> u64 {
        self.last_message_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn notify(&self, method_name: &str, data: Bytes) {
        let message_id = self.next_message_id();
        let header = format!("1 {} {}", message_id, method_name);
        let capacity = header.len() + 1 + data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !data.is_empty() {
            buf.put_slice(b" ");
            buf.put(data);
        }
        self.send(buf.into());
    }
    pub fn request(&self, method_name: &str, data: Bytes) -> Response {
        // FIXME except for the message type this is the exactly the same code
        // as notify.
        let message_id = self.next_message_id();
        let header = format!("2 {} {}", message_id, method_name);
        let capacity = header.len() + 1 + data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !data.is_empty() {
            buf.put_slice(b" ");
            buf.put(data);
        }
        // prepare back channel
        let (tx, rx) = oneshot::channel();
        self.requests.insert(message_id, tx);
        self.send(buf.into());
        Response {
            is_broadcast: false,
            result_rx: rx,
        }
    }
    /// This function is used to handle both notifications and requests
    fn handle_request(
        self: &Arc<Self>,
        message_id: Option<u64>,
        service: &str,
        method: &str,
        data: Bytes,
    ) {
        // FIXME add some kind of rate limiting and/or max concurrent requests
        // setting. Otherwise this could easily used for a denial of service
        // attack.
        let engine = self.clone();
        let service = service.to_owned();
        let method = method.to_owned();
        tokio::spawn(async move {
            if let Some(provider) = engine.listener.upgrade() {
                match (provider.call(service, method, data).await, message_id) {
                    (Ok(data), Some(message_id)) => engine.send_response(message_id, Ok(data)),
                    (Err(error), Some(message_id)) => engine.send_response(message_id, Err(error)),
                    (Ok(_), None) => println!("Response for notification ready. Ignoring it."),
                    (Err(_), None) => println!("Error for notification ready. Ignoring it."),
                }
            } else {
                println!("Provider not set. Did you forget to call Engine::start?");
            }
        });
    }
    /// This function is used to send both normal responses and error responses
    fn send_response(&self, request_message_id: u64, data: Result<Bytes, ProviderError>) {
        // FIXME this is almost the same code as when sending notifications and requests
        let message_id = self.next_message_id();
        let header = match data {
            Ok(_) => format!("3 {} {}", message_id, request_message_id),
            Err(_) => format!("4 {} {}", message_id, request_message_id),
        };
        let data = match data {
            Ok(data) => data,
            Err(err) => err.to_bytes(),
        };
        let capacity = header.len() + 1 + data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !data.is_empty() {
            buf.put_slice(b" ");
            buf.put_slice(&data);
        }
        self.send(buf.into());
    }
    /// This function is used to handle both normal responses and error responses
    fn handle_response(&self, request_message_id: u64, response: Result<Bytes, ConsumerError>) {
        match self.requests.remove(&request_message_id) {
            Some((_, tx)) => {
                // If the caller is no longer interrested in the response
                // it can drop the `Response` object which causes this channel
                // to be closed. This is to be expected and therefore a failing
                // send is simply ignored.
                let _ = tx.send(response);
            }
            None => {
                // FIXME log missing request id
            }
        }
    }
}

impl Engine {
    fn shutdown(&self) {
        // FIXME stop the possibly running transport
        self.requests.clear();
        // self.transport.stop();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Clear requests HashMap causing all channels to be closed.
        self.shutdown();
        println!("Engine dropped.");
    }
}

impl FrameHandler for Weak<Engine> {
    fn handle_frame(&self, data: Bytes) -> Result<(), FrameError> {
        let engine = self.upgrade().ok_or(FrameError::HandlerGone)?;
        let message = Message::parse(data.bytes()).ok_or(FrameError::ParseError)?;
        println!("Handling frame: {:?}", message);
        match message {
            Message::Heartbeat(_heartbeat) => {
                // XXX ignore heartbeats for now until reliable messaging is implemented
            }
            Message::Notification(notification) => {
                engine.handle_request(
                    None,
                    notification.service,
                    notification.method,
                    _get_data(&data, notification.data),
                );
            }
            Message::Request(request) => {
                engine.handle_request(
                    Some(request.message_id),
                    request.service,
                    request.method,
                    _get_data(&data, request.data),
                );
            }
            Message::Response(response) => {
                engine.handle_response(
                    response.request_message_id,
                    Ok(_get_data(&data, response.data)),
                );
            }
            Message::Error(error) => {
                engine.handle_response(
                    error.request_message_id,
                    Err(match error.kind {
                        // FIXME how do we handle when the server does not understand our message?
                        ErrorKind::InvalidMessage => ConsumerError::ProviderError,
                        ErrorKind::ServiceNotFound => ConsumerError::ServiceNotFound,
                        ErrorKind::MethodNotFound => ConsumerError::MethodNotFound,
                        ErrorKind::ProviderError => ConsumerError::ProviderError,
                        ErrorKind::Other(other) => ConsumerError::InvalidData(
                            data.slice((other.as_ptr() as usize - data.as_ptr() as usize)..),
                        ),
                    }),
                );
            }
            Message::Disconnect => {
                engine.shutdown();
            }
        }
        Ok(())
    }
    fn handle_disconnect(&self) {
        if let Ok(engine) = self.upgrade().ok_or(FrameError::HandlerGone) {
            engine.shutdown();
        }
    }
}

#[async_trait]
pub trait EngineListener {
    async fn call(
        &self,
        service: String,
        method: String,
        data: Bytes,
    ) -> Result<Bytes, ProviderError>;
    fn shutdown(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::transport::TransportError;
    use async_trait::async_trait;

    #[tokio::main]
    #[test]
    async fn test_response_err_broadcast() {
        use tokio::sync::oneshot;
        let (_tx, rx) = oneshot::channel();
        let response: Response = Response {
            is_broadcast: true,
            result_rx: rx,
        };
        assert!(matches!(response.await, Err(ConsumerError::Broadcast)));
    }

    #[tokio::main]
    #[test]
    async fn response_err_disconnected() {
        use tokio::sync::oneshot;
        let (tx, rx) = oneshot::channel();
        drop(tx);
        let response: Response = Response {
            is_broadcast: false,
            result_rx: rx,
        };
        assert!(matches!(response.await, Err(ConsumerError::Disconnected)));
    }

    #[tokio::main]
    #[test]
    async fn engine_response() {
        let provider = Arc::new(NoneProvider {});
        let (transport, mut remote) = bidi_channel();
        let engine = Arc::new(Engine::new(transport));
        engine.start(Arc::downgrade(&provider) as Weak<dyn EngineListener + Sync + Send>);
        let response = engine.request("get_answer", Bytes::copy_from_slice(b""));
        assert_eq!(
            remote.recv().await.unwrap(),
            Bytes::copy_from_slice(b"2 1 get_answer")
        );
        println!("Sending response...");
        remote.send(Bytes::copy_from_slice(b"3 1 1 42"));
        println!("Waiting for response...");
        assert_eq!(response.await.unwrap(), Bytes::copy_from_slice(b"42"));
    }

    struct NoneProvider {}

    #[async_trait]
    impl EngineListener for NoneProvider {
        async fn call(
            &self,
            service: String,
            method: String,
            data: Bytes,
        ) -> Result<Bytes, ProviderError> {
            Err(ProviderError::ServiceNotFound)
        }
        fn shutdown(&self) {}
    }

    struct FakeTransport {
        tx: tokio::sync::mpsc::UnboundedSender<Bytes>,
        rx: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<Bytes>>>,
    }

    #[async_trait]
    impl Transport for FakeTransport {
        fn send(&self, frame: Bytes) -> Result<(), TransportError> {
            match self.tx.send(frame) {
                Ok(()) => Ok(()),
                Err(_) => Err(TransportError::Disconnected),
            }
        }
        fn start(&self, engine: Box<dyn FrameHandler>) {
            let mut rx = self.rx.try_lock().unwrap().take().unwrap();
            tokio::spawn(async move {
                while let Some(frame) = rx.recv().await {
                    match engine.handle_frame(frame) {
                        Ok(()) => continue,
                        Err(FrameError::HandlerGone) => break,
                        Err(FrameError::ParseError) => {
                            println!("An error occured while trying to parse a client frame");
                            break;
                        }
                    }
                }
            });
        }
    }

    struct Remote {
        tx: tokio::sync::mpsc::UnboundedSender<Bytes>,
        rx: tokio::sync::mpsc::UnboundedReceiver<Bytes>,
    }

    impl Remote {
        fn send(&self, frame: Bytes) {
            self.tx.send(frame).unwrap();
        }
        async fn recv(&mut self) -> Option<Bytes> {
            self.rx.recv().await
        }
    }

    fn bidi_channel() -> (FakeTransport, Remote) {
        let (tx0, rx0) = tokio::sync::mpsc::unbounded_channel();
        let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel();
        (
            FakeTransport {
                tx: tx0,
                rx: std::sync::Mutex::new(Some(rx1)),
            },
            Remote { tx: tx1, rx: rx0 },
        )
    }
}
