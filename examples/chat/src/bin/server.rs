use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;

use ::webwire::server::hyper::MakeHyperService;
use ::webwire::server::session::{Auth, AuthError};
use ::webwire::{ConsumerError, Response, Router, Server};

use webwire_example_chat::api::chat;

struct ChatService {
    #[allow(dead_code)]
    session: Arc<Session>,
    server: Arc<Server<Session>>,
}

#[async_trait]
impl chat::Server for ChatService {
    type Error = webwire::ProviderError;
    async fn send(&self, message: &chat::ClientMessage) -> Response<Result<(), chat::SendError>> {
        let client = chat::ClientConsumer(&*self.server);
        // FIXME Using matches!() to ensure the response resolves to
        // a Err(ConsumerError::Broadcast) is far from pretty.
        let msg = chat::Message {
            username: self.session.username.clone(),
            text: message.text.clone(),
        };
        let fut = client.on_message(&msg);
        assert!(matches!(fut.await, Err(ConsumerError::Broadcast)));
        Ok(Ok(()))
    }
}

#[derive(Default)]
struct Session {
    username: String,
}

struct Sessions {}

impl Sessions {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl webwire::SessionHandler<Session> for Sessions {
    async fn auth(&self, auth: Option<Auth>) -> Result<Session, AuthError> {
        match auth {
            None => Err(AuthError::Unauthorized),
            Some(Auth::Basic {
                username,
                password: _,
            }) => Ok(Session { username }),
            Some(_) => Err(AuthError::Unsupported),
        }
    }
    async fn connect(&self, _session: &Session) {}
    async fn disconnect(&self, _session: &Session) {}
}

#[tokio::main]
async fn main() {
    // Create session handler
    let session_handler = Sessions::new();

    // Create service router
    let router = Arc::new(Router::<Session>::new());

    // Create webwire server
    let server = Arc::new(webwire::server::Server::new(
        session_handler,
        router.clone(),
    ));

    // Register services
    router.service(chat::ServerProvider({
        let server = server.clone();
        move |session| ChatService {
            session,
            server: server.clone(),
        }
    }));

    // Start hyper service
    let addr = SocketAddr::from(([0, 0, 0, 0], 2323));
    let make_service = MakeHyperService { server };
    let server = hyper::Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
