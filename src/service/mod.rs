//! Generic types and traits for services

pub mod provider;
pub use provider::{NamedProvider, Provider, ProviderError, Response, Router};

pub mod consumer;
pub use consumer::{Consumer, ConsumerError};
