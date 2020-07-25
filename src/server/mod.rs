//! Server implementation

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;

use crate::rpc::transport::Transport;
use crate::service::Router;

pub mod connection;
pub mod hyper;
pub mod session;

use connection::Connection;
use session::{Auth, AuthError, SessionHandler};

/// The webwire server which is the
pub struct Server<S: Sync + Send> {
    last_connection_id: AtomicUsize,
    connections: DashMap<usize, Arc<Connection<S>>>,
    router: Router<S>,
    session_handler: Box<dyn SessionHandler<S> + Sync + Send>,
}

impl<S: Sync + Send + 'static> Server<S> {
    /// Create a new server object using the given session handler
    /// and provider registry.
    pub fn new<H: SessionHandler<S> + Sync + Send + 'static>(
        session_handler: H,
        router: Router<S>,
    ) -> Self {
        Self {
            last_connection_id: AtomicUsize::new(0),
            connections: DashMap::new(),
            session_handler: Box::new(session_handler),
            router,
        }
    }
    /// This function is called to authenticate connections.
    pub async fn auth(&self, auth: Option<Auth>) -> Result<S, AuthError> {
        println!("Auth!");
        self.session_handler.auth(auth).await
    }
    /// This function is called when a client connects.
    pub async fn connect<T: Transport + 'static>(self: &Arc<Self>, transport: T, session: S) {
        println!("Connect!");
        let connection_id = self
            .last_connection_id
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let conn = Connection::new(connection_id, &self, transport, session);
        self.connections.insert(connection_id, conn);
    }
    /// This function is called when a client disconnects.
    pub fn disconnect(&self, connection_id: usize) {
        self.connections.remove(&connection_id);
    }
}
