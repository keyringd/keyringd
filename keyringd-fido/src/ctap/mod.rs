mod pin;
mod pin_validation_error;
pub use pin::Pin;
pub use pin_validation_error::PinValidationError;
mod authenticator_client_pin_request;
mod authenticator_client_pin_response;
mod authenticator_get_assertion_request;
mod authenticator_get_assertion_response;
mod authenticator_get_info_request;
mod authenticator_get_info_response;
mod authenticator_make_credential_request;
mod authenticator_make_credential_response;
mod client_data_hash;
mod client_pin_subcommand;
mod credential_protection_policy;
mod ctap_command;
mod ctap_status_code;
mod ctap_version;
mod pin_uv_auth_permissions;
mod pin_uv_key_agreement_key;

pub(crate) use crate::webauthn::{
    AuthenticationExtensionsAuthenticatorInputs, AuthenticationExtensionsAuthenticatorOutputs,
    PublicKeyCredentialDescriptor, PublicKeyCredentialParameters, PublicKeyCredentialRpEntity,
    PublicKeyCredentialUserEntity,
};
pub use authenticator_client_pin_request::AuthenticatorClientPinRequest;
pub use authenticator_client_pin_response::AuthenticatorClientPinResponse;
pub use authenticator_get_assertion_request::AuthenticatorGetAssertionRequest;
pub use authenticator_get_assertion_response::AuthenticatorGetAssertionResponse;
pub use authenticator_get_info_request::AuthenticatorGetInfoRequest;
pub use authenticator_get_info_response::AuthenticatorGetInfoResponse;
pub use authenticator_make_credential_request::AuthenticatorMakeCredentialRequest;
pub use authenticator_make_credential_response::AuthenticatorMakeCredentialResponse;
pub use client_data_hash::ClientDataHash;
pub use client_pin_subcommand::ClientPinSubcommand;
pub use credential_protection_policy::CredentialProtectionPolicy;
pub use ctap_command::CtapCommand;
pub use ctap_status_code::CtapStatusCode;
pub use ctap_version::CtapVersion;
pub use pin_uv_auth_permissions::PinUvAuthPermissions;
pub use pin_uv_key_agreement_key::PinUvKeyAgreementKey;

pub(crate) use crate::ProtocolError;
pub(crate) use crate::codec::CborValue as CtapValue;

mod ctap_request;
mod ctap_response;
pub use ctap_request::CtapRequest;
pub use ctap_response::CtapResponse;

#[cfg(test)]
pub(crate) use pin_uv_key_agreement_key::KEY_TYPE_EC2;

#[cfg(test)]
pub(crate) use pin_uv_key_agreement_key::ALGORITHM_ECDH_ES_HKDF_256;

#[cfg(test)]
pub(crate) use pin_uv_key_agreement_key::CURVE_P256;

mod authenticator_large_blobs_request;
mod large_blob_array;
mod large_blob_map;
pub use authenticator_large_blobs_request::AuthenticatorLargeBlobsRequest;
pub use large_blob_array::LargeBlobArray;
pub use large_blob_map::LargeBlobMap;

mod authenticator_large_blob_extension;
mod authenticator_large_blob_output;
pub use authenticator_large_blob_extension::AuthenticatorLargeBlobExtension;
pub use authenticator_large_blob_output::AuthenticatorLargeBlobOutput;

mod authenticator_large_blobs_response;
pub use authenticator_large_blobs_response::AuthenticatorLargeBlobsResponse;
