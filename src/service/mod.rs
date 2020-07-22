//! Generic types and traits for services

pub mod provider;
pub use provider::{Provider, ProviderError, Request, Response, ProviderRegistry};

pub mod consumer;
pub use consumer::{Consumer, ConsumerError};
