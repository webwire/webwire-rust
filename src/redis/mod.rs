//! Redis support
//!
//! This module implements support for Webwire RPC via
//! the redis message queue.

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
