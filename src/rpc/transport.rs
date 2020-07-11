use std::fmt;

use async_trait::async_trait;
use bytes::Bytes;

use crate::rpc::engine::Engine;

#[async_trait]
pub trait Transport {
    fn send(&self, frame: Bytes) -> Result<(), TransportError>;
    fn start(&self, engine: Engine);
}

#[derive(Debug)]
pub enum TransportError {
    Disconnected,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Disconnected => "Disconnected",
            }
        )
    }
}

impl std::error::Error for TransportError {}
