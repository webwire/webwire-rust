#![cfg(feature = "redis")]

use std::time::Duration;

use bytes::Bytes;
use redis::{Client, ConnectionInfo};
use tokio::sync::{mpsc, Mutex};
use webwire::{redis::RedisPublisher, Consumer, NamedProvider, Provider, Router};

struct FakeService {
    tx: mpsc::Sender<()>,
}

impl<S: Sync + Send> Provider<S> for FakeService {
    fn call(
        &self,
        _session: &std::sync::Arc<S>,
        service: &str,
        method: &str,
        data: bytes::Bytes,
    ) -> futures::future::BoxFuture<'static, Result<Bytes, webwire::ProviderError>> {
        assert_eq!(service, "test_service");
        assert_eq!(method, "test_method");
        assert_eq!(data, "test_data");
        let tx = self.tx.clone();
        Box::pin(async move {
            tx.send(()).await.unwrap();
            Ok(Bytes::new())
        })
    }
}

impl<S: Sync + Send> NamedProvider<S> for FakeService {
    const NAME: &'static str = "test_service";
}

#[tokio::test]
async fn test_redis() {
    let redis_url = "redis://127.0.0.1";
    let router = Router::new();
    let (tx, mut rx) = mpsc::channel(1);
    router.service(FakeService { tx });
    let listener = webwire::redis::RedisListener::new(redis_url, "test_webwire", router).unwrap();
    tokio::spawn(listener.run());
    // FIXME wait for provider to be ready
    // The easiest way to implement this feature would be a signal
    // sent by the listener whenever it is connected to redis and
    // ready to receive messages.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mgr = deadpool_redis::Manager::new(redis_url).unwrap();
    let pool = deadpool_redis::Pool::builder(mgr).build().unwrap();
    let publisher = RedisPublisher::new(pool, "test_webwire");
    publisher.notify(
        "test_service",
        "test_method",
        Bytes::from_static(b"test_data"),
    );
    rx.recv().await;
}
