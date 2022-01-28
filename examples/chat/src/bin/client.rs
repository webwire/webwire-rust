use std::sync::Arc;

use async_trait::async_trait;

use webwire::{client::Client, server::session::Auth};

use webwire_example_chat::api::chat;

struct ChatClient {
    client: Arc<Client>,
}

#[async_trait]
impl chat::Client for ChatClient {
    type Error = webwire::ProviderError;
    async fn on_message(&self, input: &chat::Message) -> Result<(), webwire::ProviderError> {
        println!("Message received: {:?}", input);
        if input.text == "!ping" {
            let server = chat::ServerConsumer(&*self.client);
            server
                .send(&chat::ClientMessage {
                    text: "!pong".to_string(),
                })
                .await
                .unwrap()
                .unwrap();
        }
        Ok(())
    }
}

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Arc::new(webwire::Router::new());
    let client = Arc::new(webwire::client::Client::new(
        router.clone(),
        Some(Auth::Basic {
            username: "john".to_string(),
            password: "".to_string(),
        }),
    ));
    router.service(chat::ClientProvider({
        let client = client.clone();
        move |_| ChatClient {
            client: client.clone(),
        }
    }));
    let server_url = "ws://127.0.0.1:2323".parse::<url::Url>()?;
    client.connect(server_url).await?;
    client.join().await;
    Ok(())
}

#[tokio::main]
async fn main() {
    _main().await.unwrap()
}
