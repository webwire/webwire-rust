use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use super::message::{self, Message};
use crate::{Consumer, ConsumerError, Server};

struct IdGenerator {
    last_id: AtomicU64,
}

impl IdGenerator {
    fn new() -> Self {
        Self { last_id: AtomicU64::new(0) }
    }
    fn next(&self) -> u64 {
        self.last_id.fetch_add(1, Ordering::Relaxed) + 1
    }
}

#[async_trait]
pub trait Transport {
    fn send(&self, data: Bytes);
    async fn recv(&self) -> Option<Bytes>;
}

pub struct Request {
    result_tx: oneshot::Sender<Bytes>,
    consumer: Consumer,
}

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

#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

pub struct EngineInner {
    id_generator: IdGenerator,
    requests: DashMap<u64, oneshot::Sender<Result<Bytes, ConsumerError>>>,
    transport: Box<dyn Transport + Sync + Send>,
}

impl Engine {
    pub fn new<T: Transport + Sync + Send + 'static>(transport: T) -> Self {
        let engine = Self {
            inner: Arc::new(EngineInner {
                id_generator: IdGenerator::new(),
                requests: DashMap::new(),
                transport: Box::new(transport),
            })
        };
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.run().await
        });
        engine
    }
    async fn run(&self) {
        while let Some(frame) = self.inner.transport.recv().await {
            self.handle_frame(frame);
        }
        // FIXME connection was lost... what now?
    }
    pub fn notify(&mut self, method_name: &str, data: Bytes) {
        let message_id = self.inner.id_generator.next();
        let header = format!("1 {} {}", message_id, method_name);
        let capacity = header.len() + 1 + data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !data.is_empty() {
            buf.put_slice(b" ");
            buf.put(data);
        }
        self.inner.transport.send(buf.into());
    }
    pub fn request(&mut self, method_name: &str, data: Bytes) -> Response {
        // FIXME except for the message type this is the exactly the same code
        // as notify.
        let message_id = self.inner.id_generator.next();
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
        self.inner.requests.insert(message_id, tx);
        self.inner.transport.send(buf.into());
        Response {
            is_broadcast: false,
            result_rx: rx,
        }
    }
    pub fn handle_frame(&self, data: Bytes) -> Option<()> {
        let message = Message::parse(data.bytes())?;
        match message {
            Message::Heartbeat(heartbeat) => {
                // XXX ignore heartbeats for now until reliable messaging is implemented
            }
            Message::Notification(notification) => {
                // FIXME implement
                //self.provider
                //self.handle_notification(notification)?;
            }
            Message::Request(request) => {
                // FIXME implement
            }
            Message::Response(response) => {
                self.handle_response(
                    response.request_message_id,
                    Ok(match response.data {
                        Some(response_data) => {
                            data.slice((response_data.as_ptr() as usize - data.as_ptr() as usize)..)
                        }
                        None => Bytes::new(),
                    }),
                );
            }
            Message::Error(error) => {
                self.handle_response(
                    error.request_message_id,
                    Err(match error.kind {
                        message::ErrorKind::ServiceNotFound => ConsumerError::ServiceNotFound,
                        message::ErrorKind::MethodNotFound => ConsumerError::MethodNotFound,
                        message::ErrorKind::ProviderError => ConsumerError::ProviderError,
                        message::ErrorKind::Other(other) => ConsumerError::InvalidData(
                            data.slice((other.as_ptr() as usize - data.as_ptr() as usize)..),
                        ),
                    }),
                );
            }
            Message::Disconnect => { /*FIXME implement */ }
        }
        Some(())
    }
    /// This function is used to handle both normal responses and error responses
    fn handle_response(&self, request_message_id: u64, response: Result<Bytes, ConsumerError>) {
        match self.inner.requests.remove(&request_message_id) {
            Some((_, tx)) => {
                tx.send(response);
            }
            None => {
                // FIXME log missing request id
            }
        }
    }
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        // Clear requests HashMap causing all channels to be closed.
        self.requests.clear();
    }
}

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
async fn test_response_err_disconnected() {
    use tokio::sync::oneshot;
    let (tx, rx) = oneshot::channel();
    drop(tx);
    let response: Response = Response {
        is_broadcast: false,
        result_rx: rx,
    };
    assert!(matches!(response.await, Err(ConsumerError::Disconnected)));
}

struct BidiChannel {
    tx: tokio::sync::mpsc::UnboundedSender<Bytes>,
    rx: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Bytes>>,
}

fn bidi_channel() -> (BidiChannel, BidiChannel) {
    let (tx0, rx0) = tokio::sync::mpsc::unbounded_channel();
    let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel();
    (
        BidiChannel { tx: tx0, rx: Mutex::new(rx1) },
        BidiChannel { tx: tx1, rx: Mutex::new(rx0) },
    )
}

#[async_trait]
impl Transport for BidiChannel {
    fn send(&self, frame: Bytes) {
        self.tx.send(frame).unwrap();
    }
    async fn recv(&self) -> Option<Bytes> {
        self.rx.lock().await.recv().await
    }
}

#[tokio::main]
#[test]
async fn test_engine_response() {
    let (transport, mut remote) = bidi_channel();
    let mut engine = Engine::new(transport);
    let response = engine.request("get_answer", Bytes::copy_from_slice(b""));
    assert_eq!(remote.recv().await.unwrap(), Bytes::copy_from_slice(b"2 1 get_answer"));
    remote.send(Bytes::copy_from_slice(b"3 1 1 42"));
    assert_eq!(response.await.unwrap(), Bytes::copy_from_slice(b"42"));
}
