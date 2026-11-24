use super::response_validation::*;
use crate::{FidoClientError, MakeCredentialResult, ResponseValidationError};
use keyringd_fido::webauthn::{
    AttestationObject, AttestationStatement, AuthenticationExtensionsClientOutputs,
    AuthenticatorAttachment, AuthenticatorAttestationResponse,
};
use keyringd_fido::{AuthenticatorTransport, ctap::AuthenticatorMakeCredentialResponse};

pub(in crate::client::operation) fn make_credential_result(
    response: AuthenticatorMakeCredentialResponse,
    client_data: &[u8],
    transport: AuthenticatorTransport,
) -> Result<MakeCredentialResult, FidoClientError> {
    let attachment = if transport == AuthenticatorTransport::internal() {
        AuthenticatorAttachment::Platform
    } else {
        AuthenticatorAttachment::CrossPlatform
    };
    validate_authenticator_extension_outputs(response.authenticator_data().extensions())?;
    validate_attestation_format_identifier(response.format())?;
    validate_authenticator_extension_outputs(response.unsigned_extension_outputs())?;
    let attested = response
        .authenticator_data()
        .attested_credential_data()
        .ok_or_else(|| {
            FidoClientError::protocol(ResponseValidationError::MissingAttestedCredentialData)
        })?;
    reject_forbidden_ec2_key_parameters(attested.credential_public_key())?;
    let raw_id = attested.credential_id().to_vec();
    let statement = match response.attestation_statement() {
        Some(value) => value.clone(),
        None if response.format() == "none" => AttestationStatement::None,
        None => {
            return Err(FidoClientError::protocol(
                keyringd_fido::ProtocolError::MissingCtapField(3),
            ));
        }
    };
    let response = AuthenticatorAttestationResponse::from_attestation_object(
        client_data.to_vec(),
        AttestationObject::new(response.authenticator_data().clone(), statement),
        vec![transport],
    )
    .map_err(FidoClientError::protocol)?;
    Ok(MakeCredentialResult::new(
        raw_id,
        response,
        Some(attachment),
        AuthenticationExtensionsClientOutputs::default(),
    ))
}
