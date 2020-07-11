use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;

pub mod rpc;
pub mod server;

#[derive(Debug)]
pub enum ProviderError {
    ServiceNotFound,
    MethodNotFound,
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    InternalError(Box<dyn std::error::Error>),
}

#[derive(Debug)]
pub enum ConsumerError {
    ServiceNotFound,
    MethodNotFound,
    SerializerError(serde_json::Error),
    DeserializerError(serde_json::Error),
    ProviderError,
    /// The data part of the message could not be understood.
    InvalidData(Bytes),
    /// The request was sent to multiple recipients therefore fetching a
    /// result is not supported.
    Broadcast,
    /// The remote side disconnected while waiting for a response
    Disconnected,
}

pub struct Request {
    pub service: String,
    pub method: String,
    pub data: Vec<u8>,
}

pub type Response<E> = Result<Vec<u8>, E>;

impl ProviderError {
    fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::ServiceNotFound => "ServiceNotFound",
            Self::MethodNotFound => "MethodNotFound",
            // FIXME This should probably be a in internal server error instead?
            Self::SerializerError(_) => "SerializerError",
            Self::DeserializerError(_) => "DeserializerError",
            Self::InternalError(_) => "InternalError",
        }
        .into()
    }
}

#[async_trait]
pub trait Consumer {
    async fn call(&self, request: &Request) -> Result<Vec<u8>, ConsumerError>;
    fn notify(&self, request: &Request) -> Result<(), ConsumerError>;
}

#[async_trait]
pub trait Provider {
    fn name(&self) -> &'static str;
    async fn call(&self, request: &Request) -> Result<Vec<u8>, ProviderError>;
    async fn notify(&self, request: &Request) -> Result<(), ProviderError>;
}
