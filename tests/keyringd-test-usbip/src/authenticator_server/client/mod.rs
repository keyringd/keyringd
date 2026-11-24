//! HTTP administration of virtual authenticators on an existing server.
//!
//! Entry point: [`AuthenticatorServerClient::new`] / [`AuthenticatorServerClient::from_endpoint`].

mod authenticator_configuration;
mod authenticator_handle;
mod authenticator_info;
mod authenticator_profile;
mod authenticator_server_client;
mod authenticator_server_error;
mod consent_kind;
mod http;
mod hybrid_advertisement;
mod large_blob_support;
mod pending_consent;

pub use authenticator_configuration::AuthenticatorConfiguration;
pub use authenticator_info::AuthenticatorInfo;
pub use authenticator_profile::AuthenticatorProfile;
pub use authenticator_server_error::AuthenticatorServerError;
pub use consent_kind::ConsentKind;
pub(crate) use http::request;
pub use hybrid_advertisement::HybridAdvertisement;
pub use large_blob_support::LargeBlobSupport;
pub use pending_consent::PendingConsent;

pub use authenticator_handle::AuthenticatorHandle;

pub use authenticator_server_client::AuthenticatorServerClient;
