use std::sync::Arc;

use tokio::sync::RwLock;

use crate::rpc::transport::Transport;
use crate::service::ServiceRegistry;
use session::{Auth, AuthError, SessionHandler};

pub mod hyper;
pub mod session;
pub mod connection;

use connection::Connection;

pub struct Server<S: Sync + Send, C: Sync + Send> {
    inner: Arc<ServerInner<S, C>>,
}

struct ServerInner<S: Sync + Send, C: Sync + Send> {
    connections: RwLock<Vec<Connection<S, C>>>,
    service_registry: ServiceRegistry<S, C>,
    session_handler: Box<dyn SessionHandler<S> + Sync + Send>,
}

impl<S: Sync + Send + 'static, C: Sync + Send + 'static> Server<S, C> {
    pub fn new<H: SessionHandler<S> + Sync + Send + 'static>(
        session_handler: H,
        service_registry: ServiceRegistry<S, C>,
    ) -> Self {
        Self {
            inner: Arc::new(ServerInner {
                connections: RwLock::new(Vec::new()),
                session_handler: Box::new(session_handler),
                service_registry,
            }),
        }
    }
    pub async fn auth(&self, auth: Option<Auth>) -> Result<S, AuthError> {
        println!("Auth!");
        self.inner.session_handler.connect(auth).await
    }
    pub async fn connect<T: Transport + 'static>(&self, transport: T, session: S) {
        println!("Connect!");
        let conn = Connection::new(self.clone(), transport, session);
        self.inner.connections.write().await.push(conn);
    }
}

impl<S: Sync + Send, C: Sync + Send> Clone for Server<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
