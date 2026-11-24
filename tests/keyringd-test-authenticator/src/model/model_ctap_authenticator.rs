use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ecdsa::signature::hazmat::PrehashSigner;
use hmac::{Hmac, KeyInit, Mac};
use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::elliptic_curve::Generate;
use sha2::{Digest, Sha256};

use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
    AuthenticatorGetInfoResponse, AuthenticatorMakeCredentialRequest,
    AuthenticatorMakeCredentialResponse,
};
use keyringd_fido::webauthn::{
    AuthenticationExtensionsAuthenticatorOutputs, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialUserEntity,
};
use keyringd_fido::{
    AttestationStatement, CredentialProtectionPolicy, CredentialPublicKey, EncodedCborValue,
};
use keyringd_fido::{AuthenticatorData, CtapCommand, CtapStatusCode};

use super::{ModelAuthenticatorError, ModelCredential};

use super::State;

use super::PendingAssertion;

/// Stateful, in-memory CTAP2 authenticator used to exercise the real client
/// codec and command sequencing without transport or platform APIs.
pub struct ModelCtapAuthenticator {
    state: Mutex<State>,
    assertion_timeout: Duration,
    pin_uv_auth_token: Option<Vec<u8>>,
}

impl Default for ModelCtapAuthenticator {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            assertion_timeout: Duration::from_secs(30),
            pin_uv_auth_token: None,
        }
    }
}

impl ModelCtapAuthenticator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_assertion_timeout(assertion_timeout: Duration) -> Self {
        Self {
            state: Mutex::new(State::default()),
            assertion_timeout,
            pin_uv_auth_token: None,
        }
    }

    pub fn with_pin_uv_auth_token(pin_uv_auth_token: Vec<u8>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            assertion_timeout: Duration::from_secs(30),
            pin_uv_auth_token: Some(pin_uv_auth_token),
        }
    }

    pub fn transmit(
        &self,
        command: CtapCommand,
        request: &[u8],
    ) -> Result<Vec<u8>, ModelAuthenticatorError> {
        {
            let mut state = self.state.lock().unwrap();
            state.commands.push(command);
            if command != CtapCommand::GetNextAssertion {
                state.pending_assertions.clear();
                state.assertion_deadline = None;
            }
        }
        match command {
            CtapCommand::GetInfo => success(get_info().to_cbor_bytes().expect("model getInfo")),
            CtapCommand::MakeCredential => self.make_credential(request),
            CtapCommand::GetAssertion => self.get_assertion(request),
            CtapCommand::GetNextAssertion => self.get_next_assertion(),
            _ => Ok(status(CtapStatusCode::InvalidCommand)),
        }
    }

    pub fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        state.pending_assertions.clear();
        state.assertion_deadline = None;
    }

    pub fn commands(&self) -> Vec<CtapCommand> {
        self.state.lock().unwrap().commands.clone()
    }

    fn make_credential(&self, encoded: &[u8]) -> Result<Vec<u8>, ModelAuthenticatorError> {
        let Ok(request) = AuthenticatorMakeCredentialRequest::from_cbor_bytes(encoded) else {
            return Ok(status(CtapStatusCode::InvalidCbor));
        };
        if !request
            .public_key_credential_parameters()
            .iter()
            .any(|parameter| {
                parameter.algorithm() == keyringd_fido::webauthn::CoseAlgorithmIdentifier::ES256
            })
        {
            return Ok(status(CtapStatusCode::UnsupportedAlgorithm));
        }
        let user_verified = match self.request_user_verified(
            request.pin_uv_auth_param(),
            request.pin_uv_auth_protocol(),
            request.client_data_hash().as_bytes(),
            request.options(),
        ) {
            Ok(value) => value,
            Err(status_code) => return Ok(status(status_code)),
        };

        let mut state = self.state.lock().unwrap();
        if request.exclude_list().is_some_and(|excluded| {
            excluded
                .iter()
                .filter_map(|descriptor| state.credentials.get(descriptor.id()))
                .any(|credential| {
                    credential.relying_party_id
                        == request
                            .relying_party()
                            .id()
                            .expect("CTAP makeCredential requires rp.id")
                        && (credential.protection
                            != CredentialProtectionPolicy::UserVerificationRequired
                            || user_verified)
                })
        }) {
            return Ok(status(CtapStatusCode::CredentialExcluded));
        }

        let protection = match requested_credential_protection(&request) {
            Ok(value) => value,
            Err(status_code) => return Ok(status(status_code)),
        };

        let signing_key = SigningKey::generate();
        let verifying_key = VerifyingKey::from(&signing_key);
        let mut credential_id = vec![0; 32];
        getrandom::fill(&mut credential_id).expect("operating-system random source");
        let public_key_bytes: [u8; 65] = verifying_key
            .to_sec1_point(false)
            .as_bytes()
            .try_into()
            .expect("uncompressed P-256 public key");
        let public_key = CredentialPublicKey::from_es256_sec1_public_key(&public_key_bytes)
            .expect("generated P-256 public key");
        let creation_order = state.next_creation_order;
        state.next_creation_order = state.next_creation_order.saturating_add(1);
        if request
            .options()
            .and_then(|options| options.get("rk"))
            .copied()
            .unwrap_or(false)
        {
            let relying_party_id = request
                .relying_party()
                .id()
                .expect("CTAP makeCredential requires rp.id");
            state.credentials.retain(|_, credential| {
                !(credential.discoverable
                    && credential.relying_party_id == relying_party_id
                    && credential.user_handle == request.user().id())
            });
        }
        state.credentials.insert(
            credential_id.clone(),
            ModelCredential {
                signing_key,
                relying_party_id: request
                    .relying_party()
                    .id()
                    .expect("CTAP makeCredential requires rp.id")
                    .to_owned(),
                user_handle: request.user().id().to_vec(),
                discoverable: request
                    .options()
                    .and_then(|options| options.get("rk"))
                    .copied()
                    .unwrap_or(false),
                protection,
                creation_order,
            },
        );
        let mut raw_authenticator_data = Vec::new();
        raw_authenticator_data.extend_from_slice(&Sha256::digest(
            request
                .relying_party()
                .id()
                .expect("CTAP makeCredential requires rp.id")
                .as_bytes(),
        ));
        let extension_output = request.extensions().and_then(|extensions| {
            extensions.values().get("credProtect").map(|_| {
                AuthenticationExtensionsAuthenticatorOutputs::default().with_value(
                    "credProtect",
                    EncodedCborValue::from_canonical_bytes(vec![protection.value()])
                        .expect("credential protection is canonical CBOR"),
                )
            })
        });
        let mut flags = 0x41;
        if user_verified {
            flags |= 0x04;
        }
        if extension_output.is_some() {
            flags |= 0x80;
        }
        raw_authenticator_data.push(flags);
        raw_authenticator_data.extend_from_slice(&0u32.to_be_bytes());
        raw_authenticator_data.extend_from_slice(&[0; 16]);
        raw_authenticator_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
        raw_authenticator_data.extend_from_slice(&credential_id);
        raw_authenticator_data
            .extend_from_slice(&public_key.to_cbor_bytes().expect("model COSE key"));
        if let Some(extension_output) = extension_output {
            raw_authenticator_data.extend_from_slice(
                &extension_output
                    .to_cbor_bytes()
                    .expect("model authenticator extension output"),
            );
        }
        let authenticator_data =
            AuthenticatorData::from_bytes(&raw_authenticator_data).expect("model authData");
        success(
            AuthenticatorMakeCredentialResponse::new(
                "none",
                authenticator_data,
                AttestationStatement::None,
            )
            .to_cbor_bytes()
            .expect("model makeCredential response"),
        )
    }

    fn get_assertion(&self, encoded: &[u8]) -> Result<Vec<u8>, ModelAuthenticatorError> {
        let Ok(request) = AuthenticatorGetAssertionRequest::from_cbor_bytes(encoded) else {
            return Ok(status(CtapStatusCode::InvalidCbor));
        };
        let user_verified = match self.request_user_verified(
            request.pin_uv_auth_param(),
            request.pin_uv_auth_protocol(),
            request.client_data_hash().as_bytes(),
            request.options(),
        ) {
            Ok(value) => value,
            Err(status_code) => return Ok(status(status_code)),
        };
        let mut state = self.state.lock().unwrap();
        let allow_list_present = request.allow_list().is_some();
        let mut matches = state
            .credentials
            .iter()
            .filter(|(id, credential)| {
                credential.relying_party_id == request.rp_id()
                    && (allow_list_present || credential.discoverable)
                    && request.allow_list().is_none_or(|allowed| {
                        allowed
                            .iter()
                            .any(|descriptor| descriptor.id() == id.as_slice())
                    }) && match credential.protection {
                    CredentialProtectionPolicy::UserVerificationOptional => true,
                    CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList => {
                        allow_list_present || user_verified
                    }
                    CredentialProtectionPolicy::UserVerificationRequired => user_verified,
                }
            })
            .map(|(id, credential)| {
                (
                    id.clone(),
                    credential.user_handle.clone(),
                    credential.signing_key.clone(),
                    credential.discoverable,
                    credential.creation_order,
                )
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Ok(status(CtapStatusCode::NoCredentials));
        }
        matches.sort_by_key(|credential| std::cmp::Reverse(credential.4));
        if allow_list_present {
            matches.truncate(1);
        }
        let count = matches.len();
        let mut assertions = VecDeque::with_capacity(count);
        for (credential_id, user_handle, signing_key, discoverable, _) in matches {
            assertions.push_back(PendingAssertion {
                credential_id,
                user_handle,
                signing_key,
                discoverable,
                relying_party_id: request.rp_id().to_owned(),
                client_data_hash: *request.client_data_hash().as_bytes(),
                user_verified,
            });
        }
        let first_assertion = assertions.pop_front().expect("non-empty assertions");
        let mut first = assertion_response(&mut state, first_assertion)?;
        if count > 1 && !allow_list_present {
            first = first.with_number_of_credentials(count as u64);
        }
        state.pending_assertions = assertions;
        state.assertion_deadline =
            (!state.pending_assertions.is_empty()).then(|| Instant::now() + self.assertion_timeout);
        success(first.to_cbor_bytes().expect("model getAssertion response"))
    }

    fn get_next_assertion(&self) -> Result<Vec<u8>, ModelAuthenticatorError> {
        let mut state = self.state.lock().unwrap();
        if state
            .assertion_deadline
            .is_none_or(|deadline| Instant::now() >= deadline)
        {
            state.pending_assertions.clear();
            state.assertion_deadline = None;
            return Ok(status(CtapStatusCode::NotAllowed));
        }
        let Some(assertion) = state.pending_assertions.pop_front() else {
            state.assertion_deadline = None;
            return Ok(status(CtapStatusCode::NotAllowed));
        };
        let response = assertion_response(&mut state, assertion)?;
        state.assertion_deadline =
            (!state.pending_assertions.is_empty()).then(|| Instant::now() + self.assertion_timeout);
        success(
            response
                .to_cbor_bytes()
                .expect("model getNextAssertion response"),
        )
    }

    fn request_user_verified(
        &self,
        parameter: Option<&[u8]>,
        protocol: Option<u64>,
        client_data_hash: &[u8],
        options: Option<&BTreeMap<String, bool>>,
    ) -> Result<bool, CtapStatusCode> {
        let Some(parameter) = parameter else {
            return Ok(options
                .and_then(|options| options.get("uv"))
                .copied()
                .unwrap_or(false));
        };
        let expected_length = match protocol {
            Some(1) => 16,
            Some(2) => 32,
            _ => return Err(CtapStatusCode::PinAuthInvalid),
        };
        let token = self
            .pin_uv_auth_token
            .as_deref()
            .ok_or(CtapStatusCode::PinAuthInvalid)?;
        if parameter.len() != expected_length {
            return Err(CtapStatusCode::PinAuthInvalid);
        }
        let mut authenticator =
            Hmac::<Sha256>::new_from_slice(token).expect("HMAC accepts keys of every length");
        authenticator.update(client_data_hash);
        if authenticator.verify_truncated_left(parameter).is_err() {
            return Err(CtapStatusCode::PinAuthInvalid);
        }
        Ok(true)
    }
}

fn assertion_response(
    state: &mut State,
    assertion: PendingAssertion,
) -> Result<AuthenticatorGetAssertionResponse, ModelAuthenticatorError> {
    state.sign_count += 1;
    let mut raw = Vec::with_capacity(37);
    raw.extend_from_slice(&Sha256::digest(assertion.relying_party_id.as_bytes()));
    raw.push(0x01 | if assertion.user_verified { 0x04 } else { 0 });
    raw.extend_from_slice(&state.sign_count.to_be_bytes());
    let authenticator_data =
        AuthenticatorData::from_bytes(&raw).expect("model assertion authenticator data");
    let mut signed = raw;
    signed.extend_from_slice(&assertion.client_data_hash);
    let signature: p256::ecdsa::Signature = assertion
        .signing_key
        .sign_prehash(&Sha256::digest(signed))?;
    let response = AuthenticatorGetAssertionResponse::new(
        Some(PublicKeyCredentialDescriptor::new(
            keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
            assertion.credential_id,
        )),
        authenticator_data,
        signature.to_der().as_bytes().to_vec(),
    );
    Ok(if assertion.discoverable {
        response.with_user(PublicKeyCredentialUserEntity::from_id(
            assertion.user_handle,
        ))
    } else {
        response
    })
}

fn get_info() -> AuthenticatorGetInfoResponse {
    AuthenticatorGetInfoResponse::new(vec!["FIDO_2_0".to_owned()], [0; 16])
        .with_extensions(vec!["credProtect".to_owned()])
        .with_options(BTreeMap::from([
            ("rk".to_owned(), true),
            ("up".to_owned(), true),
            ("uv".to_owned(), true),
        ]))
        .with_max_message_size(1200)
        .with_max_credential_count_in_list(16)
        .with_max_credential_id_length(128)
        .with_algorithms(vec![PublicKeyCredentialParameters::es256()])
}

fn requested_credential_protection(
    request: &AuthenticatorMakeCredentialRequest,
) -> Result<CredentialProtectionPolicy, CtapStatusCode> {
    let Some(value) = request
        .extensions()
        .and_then(|extensions| extensions.values().get("credProtect"))
    else {
        return Ok(CredentialProtectionPolicy::UserVerificationOptional);
    };
    match value.as_bytes() {
        [1] => Ok(CredentialProtectionPolicy::UserVerificationOptional),
        [2] => Ok(CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList),
        [3] => Ok(CredentialProtectionPolicy::UserVerificationRequired),
        _ => Err(CtapStatusCode::InvalidOption),
    }
}

fn status(value: CtapStatusCode) -> Vec<u8> {
    vec![value.as_byte()]
}

fn success(mut payload: Vec<u8>) -> Result<Vec<u8>, ModelAuthenticatorError> {
    payload.insert(0, CtapStatusCode::Ok.as_byte());
    Ok(payload)
}
