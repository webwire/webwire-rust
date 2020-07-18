use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use dashmap::DashMap;

use crate::rpc::transport::Transport;
use crate::service::ProviderRegistry;

pub mod connection;
pub mod hyper;
pub mod session;

use connection::Connection;
use session::{Auth, AuthError, Session, SessionHandler};

pub struct Server<S: Session> {
    inner: Arc<ServerInner<S>>,
}

struct ServerInner<S: Session> {
    last_connection_id: AtomicUsize,
    connections: DashMap<usize, Connection<S>>,
    providers: ProviderRegistry<S>,
    session_handler: Box<dyn SessionHandler<S> + Sync + Send>,
}

impl<S: Session + 'static> Server<S> {
    pub fn new<H: SessionHandler<S> + Sync + Send + 'static>(
        session_handler: H,
        providers: ProviderRegistry<S>,
    ) -> Self {
        Self {
            inner: Arc::new(ServerInner {
                last_connection_id: AtomicUsize::new(0),
                connections: DashMap::new(),
                session_handler: Box::new(session_handler),
                providers,
            }),
        }
    }
    pub async fn auth(&self, auth: Option<Auth>) -> Result<S, AuthError> {
        println!("Auth!");
        self.inner.session_handler.connect(auth).await
    }
    pub async fn connect<T: Transport + 'static>(&self, transport: T, session: S) {
        println!("Connect!");
        let connection_id = self
            .inner
            .last_connection_id
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let conn = Connection::new(connection_id, self.clone(), transport, session);
        self.inner.connections.insert(connection_id, conn);
    }
    pub fn disconnect(&self, connection_id: usize) {
        self.inner.connections.remove(&connection_id);
    }
}

impl<S: Session + 'static> Clone for Server<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
