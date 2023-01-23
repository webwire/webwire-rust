use std::sync::Arc;

use async_trait::async_trait;
use clap::Parser;
use tokio::io::AsyncBufReadExt;

use tokio::io::AsyncWriteExt;
use tokio::io::Stdin;
use tokio::io::Stdout;
use webwire::{client::Client, server::auth::Auth};
use webwire_example_chat::api::chat;

struct ChatClient {}

#[async_trait]
impl chat::Client for ChatClient {
    type Error = webwire::ProviderError;
    async fn on_message(&self, input: &chat::Message) -> Result<(), webwire::ProviderError> {
        println!("<{}> {}", input.username, input.text);
        Ok(())
    }
}

struct Console {
    writer: tokio::io::BufWriter<Stdout>,
    reader: tokio::io::BufReader<Stdin>,
}
impl Console {
    fn new() -> Self {
        let stdout = tokio::io::stdout();
        let writer = tokio::io::BufWriter::new(stdout);
        let stdin = tokio::io::stdin();
        let reader = tokio::io::BufReader::new(stdin);
        Self { writer, reader }
    }
    async fn write(&mut self, prompt: &str) -> Result<(), tokio::io::Error> {
        self.writer.write_all(prompt.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }
    async fn prompt(&mut self, prompt: &str) -> Result<String, tokio::io::Error> {
        if !prompt.is_empty() {
            self.write(prompt).await?;
        }
        let mut buf = String::with_capacity(1024);
        self.reader.read_line(&mut buf).await?;
        buf.truncate(buf.len() - 1);
        Ok(buf)
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    server_url: url::Url,
    username: String,
    password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // a builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(tracing::Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut console = Console::new();
    let router = Arc::new(webwire::Router::new());
    let client = Arc::new(webwire::client::Client::new(
        router.clone(),
        Some(Auth::Basic {
            username: args.username,
            password: args.password.unwrap_or_default(),
        }),
    ));
    router.service(chat::ClientProvider(|_| ChatClient {}));
    tracing::info!("Connecting to server {}", args.server_url);
    client.connect(args.server_url).await?;
    tracing::info!("Connected.");
    loop {
        let text = console.prompt("").await?;
        let server = chat::ServerConsumer(&*client);
        let message = chat::ClientMessage { text };
        // FIXME it would be nice if errors would actually
        // implement the error trait so they can be used
        // with `?`
        server.send(&message).await?.unwrap();
    }
}
