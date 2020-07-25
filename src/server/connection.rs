//! Server connection

use std::sync::{Arc, Weak};

use bytes::Bytes;
use futures::future::{ready, BoxFuture};

use super::Server;
use crate::rpc::engine::{Engine, EngineListener};
use crate::rpc::transport::Transport;
use crate::service::ProviderError;

/// This is a client currently connected to the server.
pub struct Connection<S: Sync + Send>
where
    Self: Sync + Send,
{
    id: usize,
    session: Arc<S>,
    server: Arc<Server<S>>,
    engine: Arc<Engine>,
}

impl<S: Sync + Send + 'static> Connection<S> {
    /// Create a new connection object using the given `Transport` object.
    pub fn new<T: Transport + 'static>(
        id: usize,
        server: &Arc<Server<S>>,
        transport: T,
        session: S,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            id,
            session: Arc::new(session),
            server: server.clone(),
            engine: Arc::new(Engine::new(transport)),
        });
        this
            .engine
            .start(Arc::downgrade(&this) as Weak<dyn EngineListener + Sync + Send>);
        this
    }
}

impl<S: Sync + Send + 'static> EngineListener for Connection<S> {
    fn call(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> BoxFuture<Result<Bytes, ProviderError>> {
        self.server
            .router
            .call(&self.session, service, method, data)
    }
    fn shutdown(&self) {
        self.server.disconnect(self.id);
    }
}
