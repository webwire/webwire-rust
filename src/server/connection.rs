//! Server connection

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Weak};

use bytes::Bytes;

use super::Server;
use crate::rpc::engine::{Engine, EngineListener};
use crate::rpc::transport::Transport;
use crate::service::ProviderError;

/// This is a client currently connected to the server.
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

struct ConnectionInner<S: Sync + Send>
where
    Self: Sync + Send,
{
    id: usize,
    session: Arc<S>,
    server: Server<S>,
    engine: Arc<Engine>,
}

impl<S: Sync + Send + 'static> Connection<S> {
    /// Create a new connection object using the given `Transport` object.
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

impl<S: Sync + Send + 'static> EngineListener for ConnectionInner<S> {
    fn call(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, ProviderError>> + Send>> {
        self.server
            .inner
            .router
            .call(&self.session, service, method, data)
    }
    fn shutdown(&self) {
        self.server.clone().disconnect(self.id);
    }
}
