use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{service::consumer::Response, Consumer};

use super::utils::RedisBytes;

/// Publisher for broadcasts
pub struct RedisPublisher {
    conn: Arc<Mutex<redis::aio::Connection>>,
    prefix: String,
}

impl RedisPublisher {
    /// Create new RedisPublisher
    pub fn new(conn: redis::aio::Connection, prefix: &str) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            prefix: prefix.to_owned(),
        }
    }
}

impl Consumer for RedisPublisher {
    fn request(&self, service: &str, method: &str, data: bytes::Bytes) -> Response {
        self.notify(service, method, data);
        Response::notification()
    }
    fn notify(&self, service: &str, method: &str, data: bytes::Bytes) {
        let topic = format!("{}:{}:{}", self.prefix, service, method);
        let conn = self.conn.clone();
        tokio::spawn(async move {
            let mut conn = conn.lock().await;
            redis::cmd("PUBLISH")
                .arg(topic)
                .arg(RedisBytes(data))
                .query_async::<_, ()>(&mut *conn)
                .await
                .unwrap();
        });
    }
}
