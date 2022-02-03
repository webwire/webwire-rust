use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use redis::RedisError;

use crate::{Provider, Router};

// TODO
// - Add support for status signals (connected, disconnected)
// - Add support for accessing the current status
// - Add support for stopping the listener
// - Use the RPC engine instead of using the Router directly

/// Listener for broadcasts
pub struct RedisListener {
    client: redis::Client,
    prefix: String,
    router: Router<()>,
}

impl RedisListener {
    /// Create a new RedisListener
    pub fn new(
        connection_info: impl redis::IntoConnectionInfo,
        prefix: &str,
        router: Router<()>,
    ) -> Result<Self, RedisError> {
        Ok(Self {
            prefix: prefix.to_owned(),
            client: redis::Client::open(connection_info)?,
            router,
        })
    }
    /// Run the RedisListener
    ///
    /// This function consumes the RedisListener and returns
    /// a future. Simply spawn this future via `tokio::spawn`.
    pub async fn run(self) {
        tokio::spawn(async move {
            let fake_session = Arc::new(());
            // FIXME implement auto reconnect
            let conn = self.client.get_async_connection().await.unwrap();
            let mut pubsub = conn.into_pubsub();
            for service_name in self.router.service_names() {
                // FIXME add support for custom prefixes
                pubsub
                    .psubscribe(format!("{}:{}:*", self.prefix, service_name))
                    .await
                    .unwrap();
            }
            let mut stream = pubsub.on_message();
            while let Some(msg) = stream.next().await {
                let mut tokenizer = msg.get_channel_name().rsplit(':');
                // FIXME add propery error handling
                let method = tokenizer.next().unwrap();
                // FIXME add proper error handling
                let service = tokenizer.next().unwrap();
                // FIXME copying is only needed because the internals of webwire currently
                // rely on `bytes`. A better choice would be a trait which provides access
                // to the internal bytes without needing a copy of the data.
                let payload = Bytes::copy_from_slice(msg.get_payload_bytes());
                // FIXME proper error handling
                // FIXME actually... this code should probably use the RPC engine
                self.router
                    .call(&fake_session, service, method, payload)
                    .await
                    .unwrap();
            }
        });
    }
}
