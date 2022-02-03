//! Redis support
//!
//! This module implements support for Webwire RPC via
//! the redis message queue.

// FIXME Mute the warnings for the time being. Once this
// module starts to stabilize this line should be removed.
#![allow(missing_docs)]

// FIXME add some detailed information how the actual implementation works

mod consumer;
mod listener;
mod provider;
mod publisher;
mod utils;

pub use consumer::RedisConsumer;
pub use listener::RedisListener;
pub use provider::RedisProvider;
pub use publisher::RedisPublisher;
