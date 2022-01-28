use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;

use ::webwire::amqp::{AMQPConfig, AMQPProvider};
use ::webwire::server::hyper::MakeHyperService;
use ::webwire::server::session::{Auth, AuthError};
use ::webwire::{ConsumerError, Response, Router, Server};

use webwire_amqp_example::api::chat;

struct ChatService {
    //#[allow(dead_code)]
//session: Arc<Session>,
//server: Arc<Server<Session>>,
}

#[async_trait]
impl chat::Server<()> for ChatService {
    async fn send(&self, message: &chat::ClientMessage) -> Response<Result<(), chat::SendError>> {
        println!("Server.send: {:?}", message);
        Ok(Ok(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = webwire::amqp::AMQPConfig {
        username: "webwire".to_owned(),
        password: "webwire".to_owned(),
        vhost: "webwire".to_owned(),
        exchange_name: "webwire_test_exchange".to_owned(),
        queue_name: "webwire_test_queue".to_owned(),
        ..Default::default()
    };
    let mut provider = AMQPProvider::new(config);
    provider.service(chat::ServerProvider(|_| ChatService {}));
    provider.run().await?;
    Ok(())
}
