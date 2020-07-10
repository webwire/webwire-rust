use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::Sink;
use hyper_websocket_lite::AsyncClient;
use tokio::stream::Stream;
use websocket_codec::{Message, Opcode};

use crate::rpc::transport::{Transport, TransportError};

pub struct WebsocketTransport {
    client: AsyncClient,
}

impl WebsocketTransport {
    pub fn new(client: AsyncClient) -> Self {
        Self { client }
    }
}

impl Transport for WebsocketTransport {}

impl Stream for WebsocketTransport {
    type Item = Result<Bytes, TransportError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.get_mut().client).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(message))) => match message.opcode() {
                Opcode::Text | Opcode::Binary => Poll::Ready(Some(Ok(message.into_data()))),
                _ => Poll::Pending,
            },
            // FIXME it should be possible to downcast this to the proper error type
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}

impl Sink<Bytes> for WebsocketTransport {
    type Error = TransportError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().client).poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().client).start_send(Message::new(Opcode::Binary, item).unwrap())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().client).poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().client).poll_close(cx)
    }
}
