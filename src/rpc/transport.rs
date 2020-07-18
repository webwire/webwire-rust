use std::fmt;

use async_trait::async_trait;
use bytes::Bytes;

#[async_trait]
pub trait Transport: Sync + Send {
    fn send(&self, frame: Bytes) -> Result<(), TransportError>;
    fn start(&self, frame_handler: Box<dyn FrameHandler>);
}

#[derive(Debug)]
pub enum TransportError {
    Disconnected,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Disconnected => "Disconnected",
        };
        write!(f, "{}", message)
    }
}

impl std::error::Error for TransportError {}

#[derive(Debug)]
pub enum FrameError {
    ParseError,
    HandlerGone,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ParseError => "ParseError",
            Self::HandlerGone => "HandlerGone",
        };
        write!(f, "{}", message)
    }
}

impl std::error::Error for FrameError {}

pub trait FrameHandler: Sync + Send {
    /// Handle an incoming frame
    fn handle_frame(&self, frame: Bytes) -> Result<(), FrameError>;
}
