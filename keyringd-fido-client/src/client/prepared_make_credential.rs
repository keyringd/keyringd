use crate::{
    AuthenticatorRequirements, ClientData, ClientDataProvider, FidoClientError, TransportSelection,
};
use keyringd_fido::EncodedCborValue;
use keyringd_fido::ctap::{
    AuthenticatorMakeCredentialRequest, ClientDataHash, CredentialProtectionPolicy,
};
use keyringd_fido::webauthn::{
    AttestationConveyancePreference, AuthenticationExtensionsAuthenticatorInputs,
    AuthenticatorAttachment, CoseAlgorithmIdentifier,
    CredentialProtectionPolicy as WebAuthnCredentialProtectionPolicy,
    PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRpEntity, PublicKeyCredentialType,
    ResidentKeyRequirement, UserVerificationRequirement,
};

#[derive(Debug)]
pub(crate) struct PreparedMakeCredential {
    pub(crate) options: PublicKeyCredentialCreationOptions,
    pub(crate) client_data: ClientData,
    ctap_request: AuthenticatorMakeCredentialRequest,
    requirements: AuthenticatorRequirements,
}

impl PreparedMakeCredential {
    pub(crate) fn new(
        options: PublicKeyCredentialCreationOptions,
        client_data_provider: &dyn ClientDataProvider,
    ) -> Result<Self, FidoClientError> {
        let transport = TransportSelection::from_hints(options.hints())?;
        options.validate().map_err(FidoClientError::protocol)?;
        if !Self::allows_cross_platform_authenticator(
            options
                .authenticator_selection()
                .and_then(|selection| selection.authenticator_attachment()),
        ) && transport != Some(TransportSelection::Internal)
        {
            return Err(FidoClientError::NoAuthenticator);
        }
        let client_data = client_data_provider
            .provide(options.challenge())
            .map_err(FidoClientError::protocol)?;
        let rp_id = options.rp().id().ok_or_else(|| {
            FidoClientError::UnsupportedRequirement(
                "a standalone FIDO client requires an explicit relying party ID".into(),
            )
        })?;
        let relying_party = PublicKeyCredentialRpEntity::new(options.rp().name()).with_id(rp_id);
        let parameters = Self::resolve_credential_parameters(&options)?;
        let exclude_credentials = Self::resolve_exclude_credentials(&options);
        let attestation = Self::resolve_attestation(&options);
        let options = options
            .with_public_key_credential_parameters(parameters)
            .with_exclude_credentials(exclude_credentials)
            .with_attestation(attestation);
        let resident_key = options
            .authenticator_selection()
            .map(|selection| match selection.resident_key() {
                None | Some(ResidentKeyRequirement::Unknown(_))
                    if selection.require_resident_key() =>
                {
                    ResidentKeyRequirement::Required
                }
                None | Some(ResidentKeyRequirement::Unknown(_)) => {
                    ResidentKeyRequirement::Discouraged
                }
                Some(value) => value.clone(),
            })
            .unwrap_or(ResidentKeyRequirement::Discouraged);
        if resident_key != ResidentKeyRequirement::Required {
            return Err(FidoClientError::UnsupportedRequirement(
                "keyringd requires a discoverable credential".into(),
            ));
        }
        let mut request = AuthenticatorMakeCredentialRequest::new(
            ClientDataHash::hash(client_data.as_bytes()),
            relying_party,
            options.user().clone(),
            options.public_key_credential_parameters().to_vec(),
        )
        .with_option("rk", true);
        if !options.exclude_credentials().is_empty() {
            request = request.with_exclude_list(options.exclude_credentials().to_vec());
        }
        if let Some(policy) = Self::credential_protection(&options) {
            request = request.with_extensions(
                AuthenticationExtensionsAuthenticatorInputs::default().with_value(
                    "credProtect",
                    EncodedCborValue::from_canonical_bytes(vec![policy.value()])
                        .map_err(FidoClientError::protocol)?,
                ),
            );
        }
        Ok(Self {
            requirements: Self::resolve_requirements(&options),
            options,
            client_data,
            ctap_request: request,
        })
    }

    pub(crate) fn ctap_request(&self) -> &AuthenticatorMakeCredentialRequest {
        &self.ctap_request
    }

    pub(crate) fn user_verification(&self) -> &UserVerificationRequirement {
        &self.requirements.user_verification
    }

    pub(crate) fn authenticator_requirements(&self) -> &AuthenticatorRequirements {
        &self.requirements
    }
    fn resolve_user_verification(
        options: &PublicKeyCredentialCreationOptions,
    ) -> UserVerificationRequirement {
        match options
            .authenticator_selection()
            .map(|selection| selection.user_verification())
        {
            None | Some(UserVerificationRequirement::Unknown(_)) => {
                UserVerificationRequirement::Preferred
            }
            Some(value) => value.clone(),
        }
    }

    fn resolve_requirements(
        options: &PublicKeyCredentialCreationOptions,
    ) -> AuthenticatorRequirements {
        let enforce_credential_protection = Self::credential_protection(options).is_some()
            && options
                .extensions()
                .and_then(|extensions| extensions.enforce_credential_protection_policy())
                .unwrap_or(false);
        AuthenticatorRequirements {
            discoverable_credential: true,
            user_verification: Self::resolve_user_verification(options),
            algorithms: vec![keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256],
            required_extensions: enforce_credential_protection
                .then_some("credProtect".to_owned())
                .into_iter()
                .collect(),
        }
    }

    pub(crate) fn resolve_credential_parameters(
        options: &PublicKeyCredentialCreationOptions,
    ) -> Result<Vec<PublicKeyCredentialParameters>, FidoClientError> {
        let defaults;
        let supplied = if options.public_key_credential_parameters().is_empty() {
            defaults = [PublicKeyCredentialParameters::es256()];
            defaults.as_slice()
        } else {
            options.public_key_credential_parameters()
        };

        let mut supported = Vec::new();
        for parameter in supplied.iter().filter(|parameter| {
            parameter.credential_type() == &PublicKeyCredentialType::PublicKey
                && parameter.algorithm() == CoseAlgorithmIdentifier::ES256
        }) {
            if !supported.contains(parameter) {
                supported.push(parameter.clone());
            }
        }

        if supported.is_empty() {
            return Err(FidoClientError::UnsupportedRequirement(
                "keyringd requires a public-key ES256 credential".into(),
            ));
        }

        Ok(supported)
    }

    pub(crate) fn resolve_exclude_credentials(
        options: &PublicKeyCredentialCreationOptions,
    ) -> Vec<PublicKeyCredentialDescriptor> {
        options
            .exclude_credentials()
            .iter()
            .filter(|descriptor| {
                descriptor.credential_type() == &PublicKeyCredentialType::PublicKey
            })
            .cloned()
            .collect()
    }

    fn resolve_attestation(
        options: &PublicKeyCredentialCreationOptions,
    ) -> AttestationConveyancePreference {
        match options.attestation() {
            AttestationConveyancePreference::Unknown(_) => AttestationConveyancePreference::None,
            value => value.clone(),
        }
    }

    pub(crate) fn allows_cross_platform_authenticator(
        attachment: Option<&AuthenticatorAttachment>,
    ) -> bool {
        !matches!(attachment, Some(AuthenticatorAttachment::Platform))
    }

    fn credential_protection(
        options: &PublicKeyCredentialCreationOptions,
    ) -> Option<CredentialProtectionPolicy> {
        match options.extensions()?.credential_protection_policy()? {
            WebAuthnCredentialProtectionPolicy::UserVerificationOptional => {
                Some(CredentialProtectionPolicy::UserVerificationOptional)
            }
            WebAuthnCredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList => {
                Some(CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList)
            }
            WebAuthnCredentialProtectionPolicy::UserVerificationRequired => {
                Some(CredentialProtectionPolicy::UserVerificationRequired)
            }
            WebAuthnCredentialProtectionPolicy::Unknown(_) => None,
        }
    }
}
