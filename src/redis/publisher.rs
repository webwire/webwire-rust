use crate::{service::consumer::Response, Consumer};

use super::utils::RedisBytes;

/// Publisher for broadcasts
#[derive(Clone)]
pub struct RedisPublisher {
    pool: deadpool_redis::Pool,
    prefix: String,
}

impl RedisPublisher {
    /// Create new RedisPublisher
    pub fn new(pool: deadpool_redis::Pool, prefix: &str) -> Self {
        Self {
            pool,
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
        let pool = self.pool.clone();
        let topic = format!("{}:{}:{}", self.prefix, service, method);
        tokio::spawn(async move {
            let mut conn = pool.get().await.unwrap();
            redis::cmd("PUBLISH")
                .arg(topic)
                .arg(RedisBytes(data))
                .query_async::<_, ()>(&mut conn)
                .await
                .unwrap();
        });
    }
}
