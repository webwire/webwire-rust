use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;

use dotenv::dotenv;
use serde::Deserialize;
use webwire::redis::RedisPublisher;
use webwire::server::hyper::MakeHyperService;
use webwire::server::session::{Auth, AuthError};
use webwire::{ConsumerError, Response, Router};

use webwire_example_chat::api::chat;

struct ChatService {
    #[allow(dead_code)]
    session: Arc<Session>,
    redis_publisher: RedisPublisher,
}

#[async_trait]
impl chat::Server for ChatService {
    type Error = webwire::ProviderError;
    async fn send(&self, message: &chat::ClientMessage) -> Response<Result<(), chat::SendError>> {
        let msg = chat::Message {
            username: self.session.username.clone(),
            text: message.text.clone(),
        };
        let chat_consumer = chat::ClientConsumer(&self.redis_publisher);
        let fut = chat_consumer.on_message(&msg);
        assert!(matches!(fut.await, Err(ConsumerError::Broadcast)));
        tracing::info!("<{}> {}", msg.username, msg.text);
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

#[derive(Deserialize)]
struct Config {
    redis: deadpool_redis::Config,
    listen: SocketAddr,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::new();
        cfg.merge(config::Environment::new().separator("__"))?;
        cfg.try_into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let cfg = Config::from_env().unwrap();

    // a builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(tracing::Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Create session handler
    let session_handler = Sessions::new();

    // Create service router
    let router = Arc::new(Router::<Session>::new());

    // Create webwire server
    let server = Arc::new(webwire::server::Server::new(
        session_handler,
        router.clone(),
    ));

    let redis_pool = cfg
        .redis
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
    let redis_publisher = RedisPublisher::new(redis_pool, "webwire_chat_example");

    // Register services
    router.service(chat::ServerProvider({
        let redis_publisher = redis_publisher.clone();
        move |session| ChatService {
            session,
            redis_publisher: redis_publisher.clone(),
        }
    }));

    // Start redis provider
    start_redis_listener(server.clone()).await?;

    // Start hyper service
    let make_service = MakeHyperService { server };
    let server = hyper::Server::bind(&cfg.listen).serve(make_service);

    tracing::info!("Listening on ws://{}", cfg.listen);

    if let Err(e) = server.await {
        tracing::error!("server error: {}", e);
    }

    Ok(())
}

struct RedisChat {
    server: Arc<webwire::server::Server<Session>>,
}

#[async_trait]
impl chat::Client for RedisChat {
    type Error = webwire::ProviderError;
    async fn on_message(&self, input: &chat::Message) -> Result<(), Self::Error> {
        let client = chat::ClientConsumer(&*self.server);
        let fut = client.on_message(input);
        assert!(matches!(fut.await, Err(ConsumerError::Broadcast)));
        Ok(())
    }
}

async fn start_redis_listener(
    server: Arc<webwire::server::Server<Session>>,
) -> Result<(), redis::RedisError> {
    let router = Router::new();
    router.service(chat::ClientProvider(move |_| RedisChat {
        server: server.clone(),
    }));
    let redis_listener =
        webwire::redis::RedisListener::new("redis://127.0.0.1", "webwire_chat_example", router)?;
    tokio::spawn(redis_listener.run());
    Ok(())
}
