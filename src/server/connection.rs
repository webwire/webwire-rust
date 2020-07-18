use std::sync::{Arc, Weak};

use async_trait::async_trait;
use bytes::Bytes;

use super::Server;
use crate::rpc::engine::Engine;
use crate::rpc::transport::Transport;
use crate::service::{Provider, ProviderError, Request};

pub struct Connection<S: Sync + Send>
where
    Self: Sync + Send,
{
    inner: Arc<ConnectionInner<S>>,
}

impl<S: Sync + Send> Clone for Connection<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ConnectionInner<S: Sync + Send>
where
    Self: Sync + Send,
{
    session: Arc<S>,
    server: Server<S>,
    engine: Arc<Engine>,
}

impl<S: Sync + Send + 'static> Connection<S> {
    pub fn new<T: Transport + 'static>(server: Server<S>, transport: T, session: S) -> Self {
        let session = Arc::new(session);
        let inner = Arc::new(ConnectionInner {
            session: session.clone(),
            server: server.clone(),
            engine: Arc::new(Engine::new(transport)),
        });
        inner.engine.start(Arc::downgrade(&inner) as Weak<dyn Provider + Sync + Send>);
        Self { inner }
    }
}

#[async_trait]
impl<S: Sync + Send> Provider for ConnectionInner<S> {
    async fn call(&self, request: &Request) -> Result<Bytes, ProviderError> {
        self.server
            .inner
            .service_registry
            .get(&request.service, &self.session)
            .ok_or(ProviderError::ServiceNotFound)?
            .call(request)
            .await
    }
}
