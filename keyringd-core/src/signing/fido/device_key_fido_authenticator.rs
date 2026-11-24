use crate::model::device_key::DeviceKey;
use crate::model::device_key::LocalAuthentication;
use crate::service::DeviceKeyService;
use crate::service::DeviceKeySigningRequest;
use keyringd_fido::{
    AuthenticatorData, CtapStatusCode,
    ctap::{
        AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
        AuthenticatorMakeCredentialRequest, AuthenticatorMakeCredentialResponse,
    },
    webauthn::{PublicKeyCredentialDescriptor, PublicKeyCredentialType},
};
use keyringd_fido_client::{FidoClientError, InternalAuthenticator};
use sha2::{Digest, Sha256};

/// Implements getAssertion for one selected, non-discoverable Device Key credential.
pub(crate) struct DeviceKeyFidoAuthenticator<'a, D: DeviceKeyService> {
    device_key_service: &'a D,
    device_key: &'a DeviceKey,
    context: &'a D::OperationContext,
    rp_id: &'a str,
    display_name: &'a str,
    credential: PublicKeyCredentialDescriptor,
}

impl<'a, D: DeviceKeyService> DeviceKeyFidoAuthenticator<'a, D> {
    pub(crate) fn new(
        device_key_service: &'a D,
        device_key: &'a DeviceKey,
        context: &'a D::OperationContext,
        rp_id: &'a str,
        display_name: &'a str,
    ) -> Self {
        let credential = PublicKeyCredentialDescriptor::new(
            PublicKeyCredentialType::PublicKey,
            Sha256::digest(device_key.certificate().public_key().to_bytes()).to_vec(),
        );
        Self {
            device_key_service,
            device_key,
            context,
            rp_id,
            display_name,
            credential,
        }
    }

    pub(crate) fn rp_id(&self) -> &str {
        self.rp_id
    }
    pub(crate) fn credential(&self) -> &PublicKeyCredentialDescriptor {
        &self.credential
    }
}

impl<D: DeviceKeyService> InternalAuthenticator for DeviceKeyFidoAuthenticator<'_, D> {
    async fn make_credential(
        &self,
        _: &AuthenticatorMakeCredentialRequest,
        _: tokio_util::sync::CancellationToken,
    ) -> Result<AuthenticatorMakeCredentialResponse, FidoClientError> {
        Err(CtapStatusCode::InvalidCommand.into())
    }

    async fn get_assertion(
        &self,
        request: &AuthenticatorGetAssertionRequest,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> Result<AuthenticatorGetAssertionResponse, FidoClientError> {
        if cancellation.is_cancelled() {
            return Err(FidoClientError::Cancelled);
        }
        // This authenticator does not implement ClientPIN or authenticator extensions.
        if request.pin_uv_auth_param().is_some() || request.pin_uv_auth_protocol().is_some() {
            return Err(CtapStatusCode::InvalidParameter.into());
        }
        if request
            .extensions()
            .is_some_and(|extensions| !extensions.is_empty())
        {
            return Err(CtapStatusCode::UnsupportedExtension.into());
        }
        if request
            .options()
            .is_some_and(|options| options.contains_key("rk"))
        {
            return Err(CtapStatusCode::UnsupportedOption.into());
        }
        // Unknown option keys are ignored, as required by CTAP 2.2 section 6.2.2.
        // A non-discoverable credential requires a matching allowList entry.
        if request.rp_id() != self.rp_id
            || !request.allow_list().is_some_and(|credentials| {
                credentials.iter().any(|descriptor| {
                    descriptor.credential_type() == self.credential.credential_type()
                        && descriptor.id() == self.credential.id()
                })
            })
        {
            return Err(CtapStatusCode::NoCredentials.into());
        }
        let require_up = request
            .options()
            .and_then(|options| options.get("up"))
            .copied()
            .unwrap_or(true);
        let require_uv = request
            .options()
            .and_then(|options| options.get("uv"))
            .copied()
            .unwrap_or(false);
        let digest = |authentication| {
            let mut signing_data = authenticator_data(self.rp_id, authentication)
                .raw()
                .to_vec();
            signing_data.extend_from_slice(request.client_data_hash().as_bytes());
            Sha256::digest(signing_data).into()
        };
        let signed = self
            .device_key_service
            .sign(
                self.context,
                DeviceKeySigningRequest {
                    handle: self.device_key.handle(),
                    profile_name: self.display_name,
                    digest: &digest,
                },
            )
            .await
            .map_err(|error| {
                if self.device_key_service.is_user_cancellation(&error) {
                    FidoClientError::Cancelled
                } else {
                    FidoClientError::backend(error)
                }
            })?;
        if cancellation.is_cancelled() {
            return Err(FidoClientError::Cancelled);
        }
        let data = authenticator_data(self.rp_id, signed.local_authentication);
        // Never return a successful response unless actual authentication meets the request.
        if require_uv && !data.user_verified() {
            return Err(CtapStatusCode::InvalidOption.into());
        }
        if require_up && !data.user_present() {
            return Err(CtapStatusCode::OperationDenied.into());
        }
        // CTAP ES256 signatures are ASN.1 DER; OS services return raw r || s.
        let signature = p256::ecdsa::Signature::from_slice(&signed.signature)
            .map_err(|error| FidoClientError::Protocol(Box::new(error)))?
            .to_der()
            .as_bytes()
            .to_vec();
        Ok(AuthenticatorGetAssertionResponse::new(
            Some(self.credential.clone()),
            data,
            signature,
        ))
    }
}

fn authenticator_data(rp_id: &str, authentication: LocalAuthentication) -> AuthenticatorData {
    let flags = match authentication {
        LocalAuthentication::NotRequired => 0,
        LocalAuthentication::Required => 0x05,
    };
    // This embedded authenticator has no signature counter. Protected keys require
    // local UV on every signature even when the request does not explicitly ask for it.
    AuthenticatorData::new(rp_id, flags, 0)
        .expect("UP/UV flags without extensions form valid assertion data")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::DeviceKeySignature;
    use crate::test_support::MockDeviceKeyService;
    use crate::test_support::ModelProfileFixture;
    use keyringd_fido::{
        ctap::ClientDataHash,
        webauthn::{PublicKeyCredentialDescriptor, PublicKeyCredentialType},
    };

    keyringd_testkit::scenario! {
        group "DeviceKeyFidoAuthenticator::get_assertion" {
            case "Signs CTAP clientDataHash with RP-bound authenticator data and returns a DER signature" {
                async: tokio;
                verify "CTAP fields determine the signed bytes and the response preserves the credential" {
                    for (authentication, rp_id, client_data, expected_digest) in [
                        (LocalAuthentication::NotRequired, "ssh:keyringd", b"message".as_slice(), [0x66, 0xa2, 0x38, 0xf2, 0x8b, 0x59, 0x59, 0xdc, 0x1f, 0x8c, 0x89, 0x82, 0xce, 0x44, 0xe2, 0x89, 0x2c, 0xfb, 0xff, 0x2c, 0x6d, 0xd4, 0x85, 0x6a, 0x3b, 0xef, 0xdf, 0xea, 0x23, 0x05, 0x48, 0x6e]),
                        (LocalAuthentication::Required, "ssh:keyringd", b"message".as_slice(), [0x8f, 0x58, 0xbd, 0x78, 0x95, 0x3c, 0x23, 0x45, 0x0e, 0x40, 0xf8, 0xf2, 0x7c, 0xa3, 0xda, 0xea, 0xd3, 0xd8, 0x59, 0x3f, 0xfc, 0xc1, 0x6d, 0x72, 0x8c, 0xa1, 0x57, 0xd5, 0xed, 0x7f, 0x4c, 0x6b]),
                        (LocalAuthentication::Required, "ssh:other", b"message".as_slice(), [0xd9, 0xea, 0xf7, 0x45, 0x4d, 0x73, 0xcd, 0x11, 0x5e, 0xc0, 0xea, 0xe4, 0x2e, 0x06, 0x09, 0x8a, 0xf8, 0xac, 0xec, 0x15, 0x75, 0xf3, 0xe6, 0x84, 0x53, 0x98, 0xb2, 0x81, 0xe5, 0xa4, 0xa0, 0xc1]),
                        (LocalAuthentication::Required, "ssh:keyringd", b"other message".as_slice(), [0x7f, 0xe0, 0x9d, 0x43, 0x52, 0x33, 0xe4, 0x59, 0x59, 0xbe, 0xa8, 0x0c, 0xdd, 0x9c, 0xdf, 0x83, 0xb4, 0x43, 0x72, 0xba, 0x9b, 0x07, 0x8b, 0x38, 0x16, 0xc1, 0xe3, 0x6b, 0xf2, 0x2d, 0xc1, 0x18]),
                    ] {
                        let mut service = MockDeviceKeyService::new();
                        let profile = ModelProfileFixture::default().build();
                        let device_key = profile.device_key();
                        let expected_handle = device_key.handle().clone();
                        service.expect_sign().times(1)
                            .withf(move |_, request| request.handle.as_ref() == expected_handle.as_ref()
                                && request.profile_name == "Work"
                                && (request.digest)(authentication) == expected_digest)
                            .returning(move |_, _| Ok(DeviceKeySignature { signature: vec![1; 64], local_authentication: authentication }));
                        let authenticator = DeviceKeyFidoAuthenticator::new(&service, device_key, &(), rp_id, "Work");
                        let credential = authenticator.credential().clone();
                        let request = AuthenticatorGetAssertionRequest::new(rp_id, ClientDataHash::hash(client_data))
                            .with_allow_list(vec![credential.clone()]).with_option("up", false);
                        let response = authenticator.get_assertion(&request, Default::default()).await.unwrap();
                        assert_eq!(response.credential(), Some(&credential));
                        assert_ne!(response.credential().unwrap().id(), device_key.handle().as_ref());
                        assert_eq!(response.authenticator_data().flags(), if authentication == LocalAuthentication::Required { 5 } else { 0 });
                        assert_eq!(response.authenticator_data().sign_count(), 0);
                        assert_eq!(response.authenticator_data().raw().len(), 37);
                        assert_eq!(p256::ecdsa::Signature::from_der(response.signature()).unwrap().to_bytes().as_slice(), &[1; 64]);
                    }
                }
            }
            case "Rejects another RP unknown credentials discovery and unsupported operations before signing" {
                async: tokio;
                verify "CTAP request checks cannot be bypassed by selecting an OS key" {
                    let service = MockDeviceKeyService::new();
                    let profile = ModelProfileFixture::default().build();
                    let authenticator = DeviceKeyFidoAuthenticator::new(&service, profile.device_key(), &(), "ssh:keyringd", "Work");
                    let credential = authenticator.credential().clone();
                    let request = AuthenticatorGetAssertionRequest::new("ssh:keyringd", ClientDataHash::new([1; 32]))
                        .with_allow_list(vec![credential.clone()]).with_option("up", false);
                    let other_rp = AuthenticatorGetAssertionRequest::new("ssh:other", *request.client_data_hash()).with_allow_list(vec![credential.clone()]);
                    let other_key = request.clone().with_allow_list(vec![PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![9])]);
                    let discovery = AuthenticatorGetAssertionRequest::new("ssh:keyringd", *request.client_data_hash());
                    for rejected in [other_rp, other_key, discovery] {
                        assert!(matches!(authenticator.get_assertion(&rejected, Default::default()).await, Err(FidoClientError::Ctap(CtapStatusCode::NoCredentials))));
                    }
                    assert!(matches!(authenticator.get_assertion(&request.clone().with_option("rk", false), Default::default()).await, Err(FidoClientError::Ctap(CtapStatusCode::UnsupportedOption))));
                    assert!(matches!(authenticator.get_assertion(&request.with_pin_uv_auth(vec![1], 2), Default::default()).await, Err(FidoClientError::Ctap(CtapStatusCode::InvalidParameter))));
                }
            }
            case "Does not return an assertion when requested presence or verification did not occur" {
                async: tokio;
                verify "an unprotected key cannot satisfy UP or UV merely because it can sign" {
                    let mut service = MockDeviceKeyService::new();
                    service.expect_sign().times(2).returning(|_, _| Ok(DeviceKeySignature { signature: vec![1; 64], local_authentication: LocalAuthentication::NotRequired }));
                    let profile = ModelProfileFixture::default().build();
                    let authenticator = DeviceKeyFidoAuthenticator::new(&service, profile.device_key(), &(), "ssh:keyringd", "Work");
                    let credential = authenticator.credential().clone();
                    let request = AuthenticatorGetAssertionRequest::new("ssh:keyringd", ClientDataHash::new([1; 32]))
                        .with_allow_list(vec![credential.clone()]);
                    assert!(matches!(authenticator.get_assertion(&request, Default::default()).await, Err(FidoClientError::Ctap(CtapStatusCode::OperationDenied))));
                    assert!(matches!(authenticator.get_assertion(&request.with_option("up", false).with_option("uv", true), Default::default()).await, Err(FidoClientError::Ctap(CtapStatusCode::InvalidOption))));
                }
            }
        }
    }
}
