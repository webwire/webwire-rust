//! Redis support
//!
//! This module implements support for Webwire RPC via
//! the redis message queue.

// FIXME add some detailed information how the actual implementation works

mod listener;
mod publisher;
mod utils;

pub use listener::RedisListener;
pub use publisher::RedisPublisher;
