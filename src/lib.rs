//! Webwire Library for Rust

#![warn(missing_docs)]

pub mod rpc;
pub mod server;
pub mod service;
pub mod utils;

pub use {
    server::{
        session::{DefaultSessionHandler, SessionHandler},
        Server,
    },
    service::{Consumer, ConsumerError, NamedProvider, Provider, ProviderError, Response, Router},
};
