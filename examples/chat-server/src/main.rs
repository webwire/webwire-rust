use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;

use ::api::chat;
use ::api::chat::Client;

use ::webwire::server::hyper::MakeHyperService;
use ::webwire::server::session::{Auth, AuthError};
use ::webwire::{DefaultSessionHandler, ProviderError, Response, Router};

struct ChatService {
    session: Arc<Session>,
}

impl ChatService {
    fn send_message(&self, message: &chat::Message) {
        println!("Send message: {:?}", message);
        /*
        for connection in self.server.connections() {
            let connection = connection.clone();
            let message = message.clone();
            tokio::spawn(async move {
                let client = chat::ClientConsumer(&connection);
                // We don't care about error responses when notifying
                // clients.
                // XXX Maybe those errors should at least be logged?
                let _ = client.on_message(&message).await;
            });
        }
        */
    }
}

#[async_trait]
impl chat::Server<Session> for ChatService {
    async fn send(&self, data: &chat::Message) -> Response<Result<(), chat::SendError>> {
        self.send_message(data);
        Ok(Ok(()))
    }
}

#[derive(Default)]
struct Session {}

struct Sessions {}

impl Sessions {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl webwire::SessionHandler<Session> for Sessions {
    async fn auth(&self, auth: Option<Auth>) -> Result<Session, AuthError> {
        Ok(Session::default())
    }
    async fn connect(&self, session: &Session) {}
    async fn disconnect(&self, session: &Session) {}
}

#[tokio::main]
async fn main() {
    // Create session handler
    let session_handler = Sessions::new();

    // Create services
    let mut router = Router::<Session>::new();
    router.service(chat::ServerProvider(|session| ChatService { session }));

    // Create webwire server
    let server = webwire::server::Server::new(session_handler, router);

    // Start hyper service
    let addr = SocketAddr::from(([127, 0, 0, 1], 2323));
    let make_service = MakeHyperService { server };
    let server = hyper::Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
