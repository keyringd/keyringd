use keyringd_fido::AuthenticatorData;
use keyringd_fido::CtapStatusCode;
use keyringd_fido::EncodedCborValue;
use keyringd_fido::ctap::AuthenticatorGetAssertionRequest;
use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
use keyringd_fido::ctap::AuthenticatorMakeCredentialRequest;
use keyringd_fido::ctap::AuthenticatorMakeCredentialResponse;
use keyringd_fido::ctap::ClientDataHash;
use keyringd_fido::webauthn::AuthenticationExtensionsAuthenticatorInputs;
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use keyringd_fido::webauthn::PublicKeyCredentialRpEntity;
use keyringd_fido::webauthn::PublicKeyCredentialUserEntity;
use p256::ecdsa::Signature;
use p256::ecdsa::VerifyingKey;
use p256::ecdsa::signature::hazmat::PrehashVerifier;
use sha2::Digest;
use sha2::Sha256;

pub(crate) fn request() -> AuthenticatorGetAssertionRequest {
    AuthenticatorGetAssertionRequest::new("example.com", ClientDataHash::new([7; 32]))
}

pub(crate) fn assertion(count: Option<u64>, id: u8) -> AuthenticatorGetAssertionResponse {
    let mut authenticator_data = vec![0x11; 32];
    authenticator_data.extend_from_slice(&[1, 0, 0, 0, id]);
    let response = AuthenticatorGetAssertionResponse::new(
        Some(PublicKeyCredentialDescriptor::new(
            keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
            vec![id],
        )),
        AuthenticatorData::from_bytes(&authenticator_data).unwrap(),
        vec![0x30, id],
    );
    match count {
        Some(count) => response.with_number_of_credentials(count),
        None => response,
    }
}

pub(crate) fn success<T>(response: &T, encode: impl FnOnce(&T) -> Vec<u8>) -> Vec<u8> {
    let mut result = vec![CtapStatusCode::Ok.as_byte()];
    result.extend_from_slice(&encode(response));
    result
}

pub(crate) fn make_credential_request(
    user: u8,
    relying_party_id: &str,
    discoverable: bool,
    credential_protection: Option<u8>,
) -> AuthenticatorMakeCredentialRequest {
    let mut request = AuthenticatorMakeCredentialRequest::new(
        ClientDataHash::new([user; 32]),
        PublicKeyCredentialRpEntity::new("Example").with_id(relying_party_id),
        PublicKeyCredentialUserEntity::new(
            format!("user-{user}"),
            vec![user],
            format!("User {user}"),
        ),
        vec![PublicKeyCredentialParameters::es256()],
    )
    .with_option("rk", discoverable);
    if let Some(policy) = credential_protection {
        request = request.with_extensions(
            AuthenticationExtensionsAuthenticatorInputs::default().with_value(
                "credProtect",
                EncodedCborValue::from_canonical_bytes(vec![policy]).unwrap(),
            ),
        );
    }
    request
}

pub(crate) fn credential_material(
    response: &AuthenticatorMakeCredentialResponse,
) -> (Vec<u8>, VerifyingKey) {
    let attested = response
        .authenticator_data()
        .attested_credential_data()
        .expect("model registration contains attested credential data");
    let public_key = attested
        .credential_public_key()
        .es256_sec1_public_key()
        .expect("model credential uses ES256");
    (
        attested.credential_id().to_vec(),
        VerifyingKey::from_sec1_bytes(&public_key).expect("valid model public key"),
    )
}

pub(crate) fn verify_assertion_signature(
    assertion: &AuthenticatorGetAssertionResponse,
    client_data_hash: &ClientDataHash,
    verifying_key: &VerifyingKey,
) {
    let mut signed = assertion.authenticator_data().to_bytes().unwrap();
    signed.extend_from_slice(client_data_hash.as_bytes());
    let signature = Signature::from_der(assertion.signature()).expect("DER encoded signature");
    verifying_key
        .verify_prehash(&Sha256::digest(signed), &signature)
        .expect("assertion signature verifies over authData || clientDataHash");
}
