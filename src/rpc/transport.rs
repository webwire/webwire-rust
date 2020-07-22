//! This module contains the transport neutral types and traits.

use std::fmt;

use bytes::Bytes;

/// A transport is typically a single client-server connection but
/// could also be used to implement messaging via a message queuing
/// service.
pub trait Transport: Sync + Send {
    /// Send a single frame via this transport.
    fn send(&self, frame: Bytes) -> Result<(), TransportError>;
    /// Start this transport. The `frame_handler` is stored inside
    /// the transport and used to notify the caller about incoming
    /// messages.
    fn start(&self, frame_handler: Box<dyn FrameHandler>);
}

/// This error is used whenever something happens with the transport.
#[derive(Debug)]
pub enum TransportError {
    /// The transport has lost the connection.
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

/// This error is returned by the FrameHandler when handling frames.
#[derive(Debug)]
pub enum FrameError {
    /// The received frame could not be parsed. This usually should make
    /// the transport terminate the connection as there is little that can
    /// be done to recover from this kind of error.
    ParseError,
    /// The frame handler is no longer available thus this transport
    /// should stop calling it.
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

/// The frame handler is used for the receiving channel between the
/// transport and engine.
pub trait FrameHandler: Sync + Send {
    /// Handle an incoming frame
    fn handle_frame(&self, frame: Bytes) -> Result<(), FrameError>;
    /// The transport has been disconnected
    fn handle_disconnect(&self);
}
