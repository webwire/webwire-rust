use bytes::Bytes;
use futures::{Sink, Stream};

pub trait Transport:
    Stream<Item = Result<Bytes, TransportError>> + Sink<Bytes, Error = TransportError>
{
}

pub type TransportError = Box<dyn std::error::Error + Send + Sync + 'static>;

/*
#[derive(Debug)]
pub enum TransportError {
    Disconnected,
    Error(Box<dyn std::error::Error>),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Disconnected => "Disconnected",
        })
    }
}

impl std::error::Error for TransportError {}
*/
