//! Websocket transport

use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use websocket_codec::{Message, Opcode};
use websocket_lite::AsyncClient;

use super::{FrameError, FrameHandler, Transport, TransportError};

/// A transport based on a WebSocket connection
pub struct WebsocketTransport<T> {
    tx: mpsc::UnboundedSender<Message>,
    client: std::sync::Mutex<Option<(AsyncClient<T>, mpsc::UnboundedReceiver<Message>)>>,
}

impl<T> WebsocketTransport<T> {
    /// Create new websocket transport using an `AsyncClient` provided by
    /// the `hyper_websocket_lite` crate.
    pub fn new(client: AsyncClient<T>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            client: std::sync::Mutex::new(Some((client, rx))),
        }
    }
}

impl<T: Send + AsyncRead + AsyncWrite + 'static> Transport for WebsocketTransport<T> {
    fn send(&self, frame: Bytes) -> Result<(), TransportError> {
        self.tx
            .send(Message::binary(frame))
            .map_err(|_| TransportError::Disconnected)
    }
    ///
    /// Important: This methods panics if start() has already been called.
    fn start(&self, frame_handler: Box<dyn FrameHandler>) {
        let (client, rx) = self
            .client
            .try_lock()
            .expect("Transport::start() must only be called once.")
            .take()
            .expect("Transport::start() must only be called once.");
        let (sink, stream) = client.split();
        let sender_fut = sender(sink, rx);
        let receiver_fut = receiver(stream, self.tx.clone(), frame_handler);
        tokio::spawn(async move {
            tokio::join!(sender_fut, receiver_fut);
        });
    }
}

async fn sender<T: AsyncWrite>(
    mut sink: SplitSink<AsyncClient<T>, Message>,
    mut rx: mpsc::UnboundedReceiver<Message>,
) {
    while let Some(message) = rx.recv().await {
        if let Err(e) = sink.send(message).await {
            // FIXME
            println!("Transport error while sending: {}", e);
            break;
        }
    }
    // FIXME shutdown receiver, too.
}

async fn receiver<T: AsyncRead>(
    mut stream: SplitStream<AsyncClient<T>>,
    tx: mpsc::UnboundedSender<Message>,
    frame_handler: Box<dyn FrameHandler>,
) {
    loop {
        match stream.next().await {
            Some(Ok(message)) => match message.opcode() {
                Opcode::Ping => {
                    if tx.send(Message::pong(Bytes::default())).is_err() {
                        // Sender is gone. Time to shut down the receiver, too.
                        break;
                    }
                }
                Opcode::Binary | Opcode::Text => {
                    // FIXME log an error when the frame handler fails?
                    match frame_handler.handle_frame(message.into_data()) {
                        Ok(()) => continue,
                        Err(FrameError::HandlerGone) => break,
                        Err(FrameError::ParseError) => {
                            // Be polite. Send a close message. If the sender is already gone
                            // this is nothing unusual and to be expected.
                            let _ = tx.send(Message::close(Some((
                                1,
                                "Unable to parse frame".to_string(),
                            ))));
                            break;
                        }
                    }
                }
                Opcode::Close => break,
                Opcode::Pong => {}
            },
            Some(Err(e)) => {
                println!("Transport error while receiving: {}", e);
                break;
            }
            None => break,
        }
    }
    frame_handler.handle_disconnect();
}
