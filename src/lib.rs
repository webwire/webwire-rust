//! # Webwire library for Rust
//!
//! [![Crates.io](https://img.shields.io/crates/v/webwire)](https://crates.io/crates/webwire)
//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/webwire/webwire/Rust)](https://github.com/webwire/webwire/actions)
//!
//! [![Discord Chat](https://img.shields.io/discord/726922033039933472?label=Discord+Chat&color=%23677bc4&logo=discord&logoColor=white&style=for-the-badge)](https://discord.gg/jjD6aWG)
//!
//! ![webwire logo](https://webwire.dev/logo.svg)
//!
//! Webwire is a **contract-first API system** which features an
//! interface description language a network protocol and
//! code generator for both servers and clients.
//!
//! This repository contains the the library for writing clients and
//! servers using the Rust programming language.
//!
//! To learn more about webwire in general please visit the website:
//! [webwire.dev](https://webwire.dev/)
//!
//! ## License
//!
//! Licensed under either of
//!
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT)>
//!
//! at your option.

#![warn(missing_docs)]

#[cfg(feature = "redis")]
pub mod redis;
pub mod rpc;
pub mod server;
pub mod service;
pub mod transport;
pub mod utils;

pub use {
    server::{
        auth::{AuthHandler, DefaultSessionHandler},
        Server,
    },
    service::{Consumer, ConsumerError, NamedProvider, Provider, ProviderError, Response, Router},
};
