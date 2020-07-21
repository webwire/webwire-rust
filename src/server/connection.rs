use std::sync::{Arc, Weak};

use async_trait::async_trait;
use bytes::Bytes;

use super::session::Session;
use super::Server;
use crate::rpc::engine::{Engine, EngineListener};
use crate::rpc::transport::Transport;
use crate::service::{Provider, ProviderError, Request};

pub struct Connection<S: Session>
where
    Self: Sync + Send,
{
    inner: Arc<ConnectionInner<S>>,
}

impl<S: Session> Clone for Connection<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ConnectionInner<S: Session>
where
    Self: Sync + Send,
{
    id: usize,
    session: Arc<S>,
    server: Server<S>,
    engine: Arc<Engine>,
}

impl<S: Session + 'static> Connection<S> {
    pub fn new<T: Transport + 'static>(
        id: usize,
        server: Server<S>,
        transport: T,
        session: S,
    ) -> Self {
        let inner = Arc::new(ConnectionInner {
            id,
            session: Arc::new(session),
            server: server.clone(),
            engine: Arc::new(Engine::new(transport)),
        });
        inner
            .engine
            .start(Arc::downgrade(&inner) as Weak<dyn EngineListener + Sync + Send>);
        Self { inner }
    }
}

#[async_trait]
impl<S: Session + 'static> EngineListener for ConnectionInner<S> {
    async fn call(
        &self,
        service: String,
        method: String,
        data: Bytes,
    ) -> Result<Bytes, ProviderError> {
        let request = Request {
            service: service,
            method: method,
            session: self.session.clone(),
        };
        self.server
            .inner
            .providers
            .get(&request.service)
            .ok_or(ProviderError::ServiceNotFound)?
            .call(&request, data)
            .await
    }
    fn shutdown(&self) {
        self.server.clone().disconnect(self.id);
    }
}
