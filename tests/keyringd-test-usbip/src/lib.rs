//! Virtual AuthenticatorServer support and a userspace USB/IP HID backend.
//!
//! Entry points:
//! - [`authenticator_server::build::AuthenticatorServerBuild::prepare`]: prepare the JAR.
//! - [`authenticator_server::certificate::HybridTestCertificate::generate`]: prepare TLS material.
//! - [`authenticator_server::manager::AuthenticatorServerManager::start`]: start an owned server.
//! - [`authenticator_server::client::AuthenticatorServerClient::new`]: connect to an existing server.
//! - [`usbip_hid_backend::UsbIpHidBackend::new`]: create a FIDO HID backend.
//!
//! - [`authenticator_server`]: server lifecycle, HTTP client, and certificates.
//! - [`usbip_hid_backend`]: USB/IP HID communication for FIDO clients.
//!
//! Runners compose these modules. The HID backend depends only on USB/IP
//! endpoints and does not depend on the server manager or HTTP client.
pub mod authenticator_server;
pub mod usbip_hid_backend;
