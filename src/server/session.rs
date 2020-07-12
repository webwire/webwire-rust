use std::fmt;

use async_trait::async_trait;
use uuid::Uuid;

pub enum Auth {
    // Username + Password
    Login { username: String, password: String },
    // API Key
    Key(String),
    // Token Login (e.g. JWT)
    Bearer(String),
    // Login using an existing session. This is used to implement
    // reliable messaging. The client connects using the same session
    // ID and the last message it has received.
    Session { id: Uuid, last_message_id: u64 },
    //
    Other(String, String),
}

impl Auth {
    pub fn parse(header: &[u8]) -> Auth {
        todo!()
    }
}

#[async_trait]
pub trait SessionHandler<S: Send + Sync> {
    async fn connect(&self, auth: Option<Auth>) -> Result<S, AuthError>;
    async fn disconnect(&self, session: S);
}

#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
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

pub struct DefaultSessionHandler<S: Default + Send + Sync> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Default + Send + Sync> DefaultSessionHandler<S> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData::default(),
        }
    }
}

#[async_trait]
impl<S: Default + Send + Sync> SessionHandler<S> for DefaultSessionHandler<S> {
    async fn connect(&self, _auth: Option<Auth>) -> Result<S, AuthError> {
        Ok(S::default())
    }
    async fn disconnect(&self, _session: S) {}
}
