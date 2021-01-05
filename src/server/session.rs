//! Authentication and sessions

use std::fmt;
use std::io::Write;

use async_trait::async_trait;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

/// A parsed version of the `Authorization` header
pub enum Auth {
    /// User name + Password
    Basic {
        /// User name
        username: String,
        /// Password
        password: String,
    },
    /// API Key
    Key(String),
    /// Token Login (e.g. JWT)
    Bearer(Vec<u8>),
    /// Login using an existing session. This is used to implement
    /// reliable messaging. The client connects using the same session
    /// ID and the last message it has received.
    Session {
        /// ID of the session to be continued
        id: Uuid,
        /// The last message the client has processed
        last_message_id: u64,
    },
    /// Another authentication scheme was used.
    Other {
        /// Authentication type
        auth_type: Vec<u8>,
        /// Authencation credentials
        credentials: Vec<u8>,
    },
}

impl Auth {
    /// Parse the `Authorization` header returning an `Auth` object.
    pub fn parse(header: &[u8]) -> Option<Auth> {
        let mut parts = header.splitn(2, |c| *c == b' ');
        let auth_type = parts.next()?;
        let credentials = parts.next()?.to_owned();
        Some(match auth_type {
            b"Basic" => Auth::basic_from_base64(&credentials)?,
            b"Key" => Auth::Key(String::from_utf8(credentials).ok()?.to_owned()),
            b"Bearer" => Auth::bearer_from_base64(&credentials)?,
            b"Session" => Auth::session_from_base64(&credentials)?,
            auth_type => Auth::Other {
                auth_type: auth_type.to_owned(),
                credentials: credentials.to_owned(),
            },
        })
    }
    /// Create Auth::Basic object from base64 encoded credentials
    pub fn basic_from_base64(credentials: &[u8]) -> Option<Auth> {
        let s = base64::decode(credentials).ok()?;
        let s = String::from_utf8(s).ok()?;
        let mut it = s.splitn(2, ":");
        let username = it.next()?;
        let password = it.next().unwrap_or_default();
        Some(Auth::Basic {
            username: username.to_owned(),
            password: password.to_owned(),
        })
    }
    /// Create Auth::Bearer object from credentials
    pub fn bearer_from_base64(credentials: &[u8]) -> Option<Auth> {
        let s = base64::decode(credentials).ok()?;
        Some(Auth::Bearer(s))
    }
    /// Create Auth::Session object from credentials
    pub fn session_from_base64(credentials: &[u8]) -> Option<Auth> {
        let s = base64::decode(credentials).ok()?;
        if s.len() != 24 {
            return None;
        }
        let id = Uuid::from_slice(&s[0..16]).ok()?;
        let mut cursor = std::io::Cursor::new(&s[16..]);
        let last_message_id = cursor.read_u64::<BigEndian>().ok()?;
        Some(Auth::Session {
            id,
            last_message_id,
        })
    }
    /// Serialize the Auth object into bytes for use inside the HTTP header
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        match self {
            Self::Basic { username, password } => {
                buf.write(b"Basic ").unwrap();
                let b64 = base64::encode(format!("{}:{}", username, password));
                buf.write(b64.as_bytes()).unwrap();
            }
            Self::Key(credentials) => {
                buf.write(b"Key ").unwrap();
                buf.write(credentials.as_bytes()).unwrap();
            }
            Self::Bearer(credentials) => {
                let b64 = base64::encode(credentials);
                buf.write(b64.as_bytes()).unwrap();
            }
            Self::Session { id, last_message_id } => {
                buf.write(id.as_bytes()).unwrap();
                buf.write_u64::<BigEndian>(*last_message_id).unwrap();
            }
            Self::Other { auth_type, credentials } => {
                buf.write(auth_type).unwrap();
                buf.write(b" ").unwrap();
                buf.write(credentials).unwrap();
            }
        }
        buf
    }
    pub fn to_string(&self) -> String {
        String::from_utf8(self.to_bytes()).unwrap()
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
    /// The given authorization header was invalid and could not be parsed.
    Invalid,
    /// The requested authorization method is not supported.
    Unsupported,
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
                Self::Invalid => "Invalid",
                Self::Unsupported => "Unsupported",
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

impl<S: Default + Send + Sync> Default for DefaultSessionHandler<S> {
    fn default() -> Self {
        Self::new()
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
