//! Server module for opencode communication

pub mod client;
pub mod discovery;

pub use client::Client;
pub use discovery::{discover_server, Server};
