//! Lifecycle, HTTP administration, and TLS preparation for the Kotlin server.
//!
//! `manager` uses `client` to check health. The caller prepares certificates
//! explicitly before starting a Hybrid-enabled server.
pub mod build;
pub mod certificate;
pub mod client;
pub mod manager;

mod java;
