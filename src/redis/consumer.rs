use bytes::Bytes;
use redis::AsyncCommands;

use crate::{rpc::message::Request, service::consumer::Response, Consumer};

use super::utils::RedisBytes;

#[derive(Clone)]
pub struct RedisConsumer {
    pool: deadpool_redis::Pool,
    key: String,
}

impl RedisConsumer {
    pub fn new(pool: deadpool_redis::Pool, key: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            pool,
            key: key.to_owned(),
        })
    }
}

impl Consumer for RedisConsumer {
    fn request(&self, service: &str, method: &str, data: Bytes) -> Response {
        self.notify(service, method, data);
        Response::notification()
    }
    fn notify(&self, service: &str, method: &str, data: Bytes) {
        let pool = self.pool.clone();
        let key = self.key.clone();
        let request = Request {
            is_notification: true,
            message_id: 0,
            service: service.to_owned(),
            method: method.to_owned(),
            data,
        };
        tokio::spawn(async move {
            let mut conn = pool.get().await.unwrap();
            conn.lpush::<_, _, ()>(&key, RedisBytes(request.to_bytes()))
                .await
                .unwrap();
        });
    }
}
