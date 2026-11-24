use keyringd_fido::CredentialPublicKey;
use keyringd_fido::webauthn::{PublicKeyCredentialDescriptor, PublicKeyCredentialUserEntity};

use crate::{FidoClientError, ResponseValidationError};

pub(crate) fn validate_discoverable_user(
    allow_list: Option<&[PublicKeyCredentialDescriptor]>,
    user: Option<&PublicKeyCredentialUserEntity>,
) -> Result<(), FidoClientError> {
    if allow_list.is_none() && user.is_none() {
        return Err(FidoClientError::protocol(
            ResponseValidationError::MissingDiscoverableUserEntity,
        ));
    }
    Ok(())
}

pub(crate) fn validate_authenticator_extension_outputs(
    outputs: Option<&keyringd_fido::webauthn::AuthenticationExtensionsAuthenticatorOutputs>,
) -> Result<(), FidoClientError> {
    let Some(outputs) = outputs else {
        return Ok(());
    };
    if let Some(identifier) = outputs
        .values()
        .keys()
        .find(|identifier| !is_valid_webauthn_identifier(identifier))
    {
        return Err(FidoClientError::protocol(
            ResponseValidationError::InvalidExtensionIdentifier(identifier.clone()),
        ));
    }
    Ok(())
}

pub(crate) fn validate_attestation_format_identifier(
    identifier: &str,
) -> Result<(), FidoClientError> {
    if !is_valid_webauthn_identifier(identifier) {
        return Err(FidoClientError::protocol(
            ResponseValidationError::InvalidAttestationFormatIdentifier(identifier.to_owned()),
        ));
    }
    Ok(())
}

pub(crate) fn reject_forbidden_ec2_key_parameters(
    key: &CredentialPublicKey,
) -> Result<(), FidoClientError> {
    let CredentialPublicKey::Ec2(key) = key else {
        return Ok(());
    };
    let forbidden_parameter = if key.key_id().is_some() {
        Some(2)
    } else if key.key_operations().is_some() {
        Some(4)
    } else if key.base_initialization_vector().is_some() {
        Some(5)
    } else {
        None
    };
    if let Some(label) = forbidden_parameter {
        return Err(FidoClientError::protocol(
            ResponseValidationError::ForbiddenCredentialPublicKeyParameter(label),
        ));
    }
    Ok(())
}

pub(in crate::client::operation) fn is_valid_webauthn_identifier(identifier: &str) -> bool {
    identifier.len() <= 32
        && identifier
            .bytes()
            .all(|byte| (0x21..=0x7e).contains(&byte) && byte != b'"' && byte != b'\\')
}
