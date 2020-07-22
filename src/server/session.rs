//! Authentication and sessions

use std::fmt;

use async_trait::async_trait;
use uuid::Uuid;

/// This marker trait does not contain any functions and just
/// requires the actual session to be `Sync + Send`.
pub trait Session: Sync + Send {}

/// A parsed version of the `Authorization` header
pub enum Auth {
    /// User name + Password
    Login {
        /// User name
        username: String,
        /// Password
        password: String
    },
    /// API Key
    Key(String),
    /// Token Login (e.g. JWT)
    Bearer(String),
    /// Login using an existing session. This is used to implement
    /// reliable messaging. The client connects using the same session
    /// ID and the last message it has received.
    Session {
        /// ID of the session to be continued
        id: Uuid,
        /// The last message the client has processed
        last_message_id: u64
    },
    /// Another authentication scheme was used.
    Other(String, String),
}

impl Auth {
    /// Parse the `Authorization` header returning an `Auth` object.
    pub fn parse(_header: &[u8]) -> Auth {
        // FIXME implement this
        todo!()
    }
}

/// Handler for inbound sessions
#[async_trait]
pub trait SessionHandler<S: Send + Sync> {
    /// Authorize the incoming request returning a session on
    /// success or an `AuthError` if the remote side could not be
    /// authenticated.
    async fn auth(&self, auth: Option<Auth>) -> Result<S, AuthError>;
    /// Called when the remote side did actually connect. This is
    /// a separate function from `auth` as this handler is also
    /// used for stateless communications and it is possible that
    /// a stateful clients aborts right after authorization and
    /// never actually connects.
    async fn connect(&self, session: &S);
    /// Called when the remote side disconnected.
    async fn disconnect(&self, session: &S);
}

/// Error enum for failed authorizations
#[derive(Debug)]
pub enum AuthError {
    /// The given credentials were not accepted and thus the remote
    /// side is not authorized to connect.
    Unauthorized,
    /// An internal server happened.
    InternalServerError,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Unauthorized => "Unauthorized",
                Self::InternalServerError => "Error",
            }
        )
    }
}

impl std::error::Error for AuthError {}

/// This implementation of a `SessionHandler` creates sessions
/// using the `Default` trait of given session type.
pub struct DefaultSessionHandler<S: Default + Send + Sync> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Default + Send + Sync> DefaultSessionHandler<S> {
    /// Create a new `DefaultSessionHandler`
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData::default(),
        }
    }
}

#[async_trait]
impl<S: Default + Send + Sync> SessionHandler<S> for DefaultSessionHandler<S> {
    async fn auth(&self, _auth: Option<Auth>) -> Result<S, AuthError> {
        Ok(S::default())
    }
    async fn connect(&self, _session: &S) {}
    async fn disconnect(&self, _session: &S) {}
}
