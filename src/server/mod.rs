//! Server implementation

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;

use crate::transport::Transport;
use crate::service::consumer::{Consumer, Response};
use crate::service::provider::Provider;

pub mod connection;
pub mod hyper;
pub mod session;

use connection::Connection;
use session::{Auth, AuthError, SessionHandler};

/// The webwire server which is the
pub struct Server<S: Sync + Send> {
    last_connection_id: AtomicUsize,
    connections: DashMap<usize, Arc<Connection<S>>>,
    provider: Arc<dyn Provider<S>>,
    session_handler: Box<dyn SessionHandler<S> + Sync + Send>,
}

impl<S: Sync + Send + 'static> Server<S> {
    /// Create a new server object using the given session handler
    /// and provider registry.
    pub fn new<H: SessionHandler<S> + Sync + Send + 'static>(
        session_handler: H,
        provider: Arc<dyn Provider<S>>,
    ) -> Self {
        Self {
            last_connection_id: AtomicUsize::new(0),
            connections: DashMap::new(),
            session_handler: Box::new(session_handler),
            provider,
        }
    }
    /// Return iterator over all connections.
    pub fn connections(&self) -> ConnectionIter<S> {
        ConnectionIter {
            iter: self.connections.iter(),
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
        let connection_id = self.last_connection_id.fetch_add(1, Ordering::Relaxed) + 1;
        let conn = Connection::new(connection_id, self, transport, session);
        self.connections.insert(connection_id, conn);
    }
    /// This function is called when a client disconnects.
    pub fn disconnect(self: &Arc<Self>, connection_id: usize) {
        self.connections.remove(&connection_id);
    }
}

impl<S: Sync + Send + 'static> Consumer for Server<S> {
    fn notify(&self, service: &str, method: &str, data: Bytes) {
        for connection in self.connections() {
            connection.notify(service, method, data.clone());
        }
    }
    fn request(&self, service: &str, method: &str, data: Bytes) -> Response {
        self.notify(service, method, data);
        Response::notification()
    }
}

/// Iterator returned by the `Server::connections()` method
pub struct ConnectionIter<'a, S>
where
    S: Sync + Send,
{
    iter: dashmap::iter::Iter<
        'a,
        usize,
        Arc<Connection<S>>,
    >,
}

impl<S: Sync + Send> Iterator for ConnectionIter<'_, S> {
    type Item = Arc<Connection<S>>;
    fn next(&mut self) -> Option<Arc<Connection<S>>> {
        self.iter.next().map(|t| t.value().clone())
    }
}
