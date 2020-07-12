use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;

use super::Server;
use crate::rpc::engine::Engine;
use crate::rpc::transport::Transport;
use crate::service::{Provider, ProviderError, Request};

pub struct Connection<S: Sync + Send, C: Sync + Send>
where
    Self: Sync + Send,
{
    inner: Arc<ConnectionInner<S, C>>,
}

impl<S: Sync + Send, C: Sync + Send> Clone for Connection<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ConnectionInner<S: Sync + Send, C: Sync + Send>
where
    Self: Sync + Send,
{
    session: Arc<S>,
    server: Server<S, C>,
    engine: Engine,
}

impl<S: Sync + Send + 'static, C: Sync + Send + 'static> Connection<S, C> {
    pub fn new<T: Transport + 'static>(server: Server<S, C>, transport: T, session: S) -> Self {
        let session = Arc::new(session);
        Self {
            inner: Arc::new(ConnectionInner {
                session: session.clone(),
                server: server.clone(),
                engine: Engine::new(ConnectionProvider::new(&server, &session), transport),
            }),
        }
    }
}

pub struct ConnectionProvider<S: Sync + Send, C: Sync + Send> {
    server: Server<S, C>,
    session: Arc<S>,
}

impl<S: Sync + Send, C: Sync + Send> ConnectionProvider<S, C> {
    pub fn new(server: &Server<S, C>, session: &Arc<S>) -> Self {
        Self {
            server: server.clone(),
            session: session.clone(),
        }
    }
}

#[async_trait]
impl<S: Sync + Send, C: Sync + Send> Provider for ConnectionProvider<S, C> {
    async fn call(&self, request: &Request) -> Result<Bytes, ProviderError> {
        let server = self.server.clone();
        let session = self.session.clone();
        server
            .inner
            .service_registry
            .get(&request.service, &session)
            .ok_or(ProviderError::ServiceNotFound)?
            .call(request)
            .await
    }
}
