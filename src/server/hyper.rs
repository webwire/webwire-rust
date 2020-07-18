use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use hyper::server::conn::AddrStream;
use hyper::{header, http, Body, Request, Response, StatusCode};
use hyper_websocket_lite::{server_upgrade, AsyncClient};
use tokio::sync::mpsc;
use websocket_codec::{Message, Opcode};

use super::session::{Auth, AuthError};
use crate::rpc::transport::{FrameError, FrameHandler, Transport, TransportError};

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

async fn sender(
    mut sink: SplitSink<AsyncClient, Message>,
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

async fn receiver(
    mut stream: SplitStream<AsyncClient>,
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

async fn on_client<S: Sync + Send + 'static>(
    client: AsyncClient,
    server: super::Server<S>,
    session: S,
) {
    let transport = WebsocketTransport::new(client);
    server.connect(transport, session).await;
}

async fn upgrade<S>(
    request: Request<Body>,
    server: super::Server<S>,
) -> Result<Response<Body>, Infallible>
where
    S: Sync + Send + 'static,
{
    let auth = match request.headers().get(hyper::header::AUTHORIZATION) {
        Some(value) => Some(Auth::parse(value.as_bytes())),
        None => None,
    };
    let session = match server.auth(auth).await {
        Ok(session) => session,
        Err(AuthError::Unauthorized) => {
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Unauthorized"))
                .unwrap());
        }
        Err(AuthError::InternalServerError) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Internal Server Error"))
                .unwrap());
        }
    };
    let on_client_fut = |client| on_client(client, server, session);
    Ok(match server_upgrade(request, on_client_fut).await {
        Ok(response) => response,
        Err(e) => {
            println!("Error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }
    })
}

pub struct HyperService<S: Sync + Send> {
    pub server: crate::server::Server<S>,
}

impl<S: Send + Sync + 'static> hyper::service::Service<Request<Body>> for HyperService<S> {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        Box::pin(upgrade(request, self.server.clone()))
    }
}

pub struct MakeHyperService<S: Sync + Send> {
    pub server: crate::server::Server<S>,
}

impl<S: Send + Sync + 'static> hyper::service::Service<&AddrStream> for MakeHyperService<S> {
    type Response = HyperService<S>;
    type Error = http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _conn: &AddrStream) -> Self::Future {
        let server = self.server.clone();
        let fut = async move { Ok(HyperService { server }) };
        Box::pin(fut)
    }
}
