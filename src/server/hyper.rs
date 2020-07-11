use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use hyper_websocket_lite::AsyncClient;
use tokio::sync::mpsc;
use websocket_codec::{Message, Opcode};

use crate::rpc::engine::Engine;
use crate::rpc::transport::{Transport, TransportError};

pub struct WebsocketTransport {
    tx: mpsc::UnboundedSender<Message>,
    client: std::sync::Mutex<Option<(AsyncClient, mpsc::UnboundedReceiver<Message>)>>,
}

impl WebsocketTransport {
    pub fn new(client: AsyncClient) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            client: std::sync::Mutex::new(Some((client, rx))),
        }
    }
}

impl Transport for WebsocketTransport {
    fn send(&self, frame: Bytes) -> Result<(), TransportError> {
        self.tx
            .send(Message::binary(frame))
            .map_err(|_| TransportError::Disconnected)
    }
    ///
    /// Important: This methods panics if start() has already been called.
    fn start(&self, engine: Engine) {
        let (client, rx) = self
            .client
            .try_lock()
            .expect("Transport::start() must only be called once.")
            .take()
            .expect("Transport::start() must only be called once.");
        let (sink, stream) = client.split();
        let sender_fut = sender(sink, rx);
        let receiver_fut = receiver(stream, self.tx.clone(), engine);
        tokio::spawn(async move {
            tokio::join!(sender_fut, receiver_fut);
        });
    }
}

async fn sender(
    mut sink: SplitSink<AsyncClient, Message>,
    mut rx: mpsc::UnboundedReceiver<Message>,
) {
    while let Some(message) = rx.recv().await {
        if let Err(e) = sink.send(message).await {
            // FIXME
            println!("Transport error while sending: {}", e);
            return;
        }
    }
    // FIXME shutdown receiver, too.
}

async fn receiver(
    mut stream: SplitStream<AsyncClient>,
    tx: mpsc::UnboundedSender<Message>,
    engine: Engine,
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
                    if engine.handle_frame(message.into_data()).is_err() {
                        // Be polite. Send a close message. If the sender is already gone
                        // this is nothing unusual and to be expected.
                        let _ = tx.send(Message::close(Some((
                            1,
                            "Unable to parse frame".to_string(),
                        ))));
                        break;
                    }
                }
                Opcode::Close => break,
                Opcode::Pong => {}
            },
            Some(Err(e)) => {
                println!("Transport error while receiving: {}", e);
                return;
            }
            None => break,
        }
    }
}
