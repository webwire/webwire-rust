use std::net::SocketAddr;
use std::sync::{Arc};

use api::chat::ServerConsumer;
use async_trait::async_trait;

use ::webwire::amqp::{AMQPConfig, AMQPConsumer};
use ::webwire::server::hyper::MakeHyperService;
use ::webwire::server::session::{Auth, AuthError};
use ::webwire::{Response, Router, Server, ConsumerError};

use api::chat;

struct Session {

}

struct ChatService {
    //#[allow(dead_code)]
    //session: Arc<Session>,
    //server: Arc<Server<Session>>,
}

#[async_trait]
impl chat::Server<Session> for ChatService {
    async fn send(&self, message: &chat::ClientMessage) -> Response<Result<(), chat::SendError>> {
        /*
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
        */
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
    let consumer = AMQPConsumer::new(config);
    let server = ServerConsumer(&consumer);
    let response = server.send(&api::chat::ClientMessage {
        text: "hello world".to_owned(),
    }).await?;
    println!("{:?}", response);
    Ok(())
}
