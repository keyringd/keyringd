use super::{CredentialId, Passkey, PasskeyAttestationResponseError};
use keyringd_fido::CborItem;
use keyringd_fido::webauthn::{AttestationObject, CollectedClientData};
use keyringd_fido_client::GetAssertionResult;
use sha2::{Digest, Sha256};

/// Original registration evidence. Its presence does not imply trusted attestation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyAttestationResponse {
    #[serde(serialize_with = "crate::util::base64url_serde::serialize")]
    attestation_object: Vec<u8>,
    #[serde(serialize_with = "crate::util::base64url_serde::serialize")]
    client_data_json: Vec<u8>,
}

impl<'de> serde::Deserialize<'de> for PasskeyAttestationResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "camelCase")]
        struct PasskeyAttestationResponseJson {
            #[serde(deserialize_with = "crate::util::base64url_serde::deserialize")]
            attestation_object: Vec<u8>,
            #[serde(deserialize_with = "crate::util::base64url_serde::deserialize")]
            client_data_json: Vec<u8>,
        }

        let fields = PasskeyAttestationResponseJson::deserialize(deserializer)?;
        Self::new(fields.attestation_object, fields.client_data_json)
            .map_err(serde::de::Error::custom)
    }
}

impl PasskeyAttestationResponse {
    const LARGE_BLOB_VERSION: u64 = 1;

    pub fn new(
        attestation_object: Vec<u8>,
        client_data_json: Vec<u8>,
    ) -> Result<Self, PasskeyAttestationResponseError> {
        let value = Self {
            attestation_object,
            client_data_json,
        };
        let object = value.object()?;
        let data = object.authenticator_data();
        let client = CollectedClientData::from_json_bytes(&value.client_data_json)?;
        if data.attested_credential_data().is_none()
            || data.rp_id_hash() != Sha256::digest(Passkey::RP_ID.as_bytes()).as_slice()
            || client.ceremony_type() != "webauthn.create"
            || client.origin().to_string() != Passkey::WEB_AUTHN_ORIGIN
            || client.cross_origin() == Some(true)
        {
            return Err(PasskeyAttestationResponseError::Context);
        }
        p256::PublicKey::from_sec1_bytes(&value.public_key()?)
            .map_err(|_| PasskeyAttestationResponseError::Context)?;
        Ok(value)
    }
    pub fn attestation_object(&self) -> &[u8] {
        &self.attestation_object
    }
    pub fn client_data_json(&self) -> &[u8] {
        &self.client_data_json
    }
    pub fn object(&self) -> Result<AttestationObject, PasskeyAttestationResponseError> {
        Ok(AttestationObject::from_cbor_bytes(
            &self.attestation_object,
        )?)
    }
    pub(crate) fn public_key(&self) -> Result<Vec<u8>, PasskeyAttestationResponseError> {
        Ok(self
            .object()?
            .authenticator_data()
            .attested_credential_data()
            .ok_or(PasskeyAttestationResponseError::Context)?
            .credential_public_key()
            .es256_sec1_public_key()?
            .to_vec())
    }
    pub(crate) fn matches(&self, id: &CredentialId, key: &[u8]) -> bool {
        let Ok(object) = self.object() else {
            return false;
        };
        let Some(attested) = object.authenticator_data().attested_credential_data() else {
            return false;
        };
        attested.credential_id() == id.as_bytes()
            && p256::PublicKey::from_sec1_bytes(key).ok()
                == self
                    .public_key()
                    .ok()
                    .and_then(|q| p256::PublicKey::from_sec1_bytes(&q).ok())
    }
    pub(crate) fn verify_assertion(
        &self,
        assertion: &GetAssertionResult,
    ) -> Result<Vec<u8>, PasskeyAttestationResponseError> {
        use p256::ecdsa::signature::Verifier;
        let key = self.public_key()?;
        if assertion.response().authenticator_data().rp_id_hash()
            != Sha256::digest(Passkey::RP_ID.as_bytes()).as_slice()
        {
            return Err(PasskeyAttestationResponseError::Context);
        }
        if !self.matches(
            &CredentialId::new(assertion.credential().id().to_vec()),
            &key,
        ) {
            return Err(PasskeyAttestationResponseError::Context);
        }
        let data = assertion.signed_data();
        let verifier = p256::ecdsa::VerifyingKey::from_sec1_bytes(&key)
            .map_err(|_| PasskeyAttestationResponseError::Signature)?;
        let signature = p256::ecdsa::Signature::from_der(assertion.response().signature())
            .map_err(|_| PasskeyAttestationResponseError::Signature)?;
        verifier
            .verify(&data, &signature)
            .map_err(|_| PasskeyAttestationResponseError::Signature)?;
        Ok(key)
    }
    pub(crate) fn to_large_blob(&self) -> Result<Vec<u8>, PasskeyAttestationResponseError> {
        Ok(CborItem::Array(vec![
            CborItem::Unsigned(Self::LARGE_BLOB_VERSION),
            CborItem::ByteString(self.attestation_object.clone()),
            CborItem::ByteString(self.client_data_json.clone()),
        ])
        .into_cbor_bytes()?)
    }

    pub(crate) fn from_large_blob(bytes: &[u8]) -> Result<Self, PasskeyAttestationResponseError> {
        let CborItem::Array(items) = CborItem::from_cbor_bytes(bytes)? else {
            return Err(PasskeyAttestationResponseError::StorageFormat);
        };
        match <[CborItem; 3]>::try_from(items) {
            Ok(
                [
                    CborItem::Unsigned(Self::LARGE_BLOB_VERSION),
                    CborItem::ByteString(attestation_object),
                    CborItem::ByteString(client_data_json),
                ],
            ) => Self::new(attestation_object, client_data_json),
            _ => Err(PasskeyAttestationResponseError::StorageFormat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido::{
        AuthenticatorData, CredentialPublicKey,
        webauthn::{
            AttestationStatement, AuthenticatorTransport, PublicKeyCredentialDescriptor,
            PublicKeyCredentialType,
        },
    };
    use keyringd_fido_client::{ClientData, GetAssertionsResult};
    use p256::ecdsa::{SigningKey, signature::Signer};

    fn evidence(key: &SigningKey, id: &[u8]) -> PasskeyAttestationResponse {
        let q = key.verifying_key().to_sec1_point(false);
        let mut auth = Sha256::digest(Passkey::RP_ID.as_bytes()).to_vec();
        auth.push(0x45);
        auth.extend_from_slice(&0u32.to_be_bytes());
        auth.extend_from_slice(&[0; 16]);
        auth.extend_from_slice(&(id.len() as u16).to_be_bytes());
        auth.extend_from_slice(id);
        auth.extend_from_slice(
            &CredentialPublicKey::from_es256_sec1_public_key(q.as_bytes().try_into().unwrap())
                .unwrap()
                .to_cbor_bytes()
                .unwrap(),
        );
        PasskeyAttestationResponse::new(
            AttestationObject::new(
                AuthenticatorData::from_bytes(&auth).unwrap(),
                AttestationStatement::None,
            )
            .to_cbor_bytes()
            .unwrap(),
            CollectedClientData::create(&[1; 32], Passkey::WEB_AUTHN_ORIGIN)
                .unwrap()
                .to_json_bytes()
                .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn v1_encodes_raw_byte_strings_and_rejects_unknown_versions() {
        let attestation = PasskeyAttestationResponse {
            attestation_object: vec![0xa0],
            client_data_json: b"{}".to_vec(),
        };
        let mut bytes = attestation.to_large_blob().unwrap();
        assert_eq!(bytes, [0x83, 1, 0x41, 0xa0, 0x42, b'{', b'}']);
        bytes[1] = 2;
        assert!(matches!(
            PasskeyAttestationResponse::from_large_blob(&bytes),
            Err(PasskeyAttestationResponseError::StorageFormat)
        ));
    }

    #[test]
    fn recovered_evidence_must_match_the_credential_and_verify_the_fresh_assertion() {
        let key = SigningKey::from_slice(&[3; 32]).unwrap();
        let other = SigningKey::from_slice(&[4; 32]).unwrap();
        let client_data = ClientData::from_bytes(vec![1, 2, 3]);
        let auth = AuthenticatorData::new(Passkey::RP_ID, 5, 9).unwrap();
        let mut signed = auth.raw().to_vec();
        signed.extend_from_slice(&Sha256::digest(client_data.as_bytes()));
        let signature: p256::ecdsa::Signature = key.sign(&signed);
        let response = keyringd_fido::ctap::AuthenticatorGetAssertionResponse::new(
            Some(PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                vec![7],
            )),
            auth,
            signature.to_der().as_bytes().to_vec(),
        );
        let assertion = GetAssertionsResult::from_ctap(
            vec![response],
            client_data,
            AuthenticatorTransport::usb(),
            None,
        )
        .unwrap()
        .select(None)
        .unwrap();
        let valid = evidence(&key, &[7]);
        assert!(valid.verify_assertion(&assertion).is_ok());
        assert!(matches!(
            evidence(&other, &[7]).verify_assertion(&assertion),
            Err(PasskeyAttestationResponseError::Signature)
        ));
        assert!(matches!(
            evidence(&key, &[8]).verify_assertion(&assertion),
            Err(PasskeyAttestationResponseError::Context)
        ));
        let passkey = Passkey::new(
            keyringd_ssh_agent_protocol::SshSignatureAlgorithm::SkEcdsaSha2NistP256,
            CredentialId::new(vec![7]),
            vec![1; 32],
            key.verifying_key().to_sec1_point(false).as_bytes().to_vec(),
        )
        .unwrap()
        .with_attestation_response(valid.clone())
        .unwrap();
        let mut encoded = serde_json::to_value(passkey).unwrap();
        encoded["attestationResponse"] = serde_json::to_value(evidence(&other, &[7])).unwrap();
        assert!(serde_json::from_value::<Passkey>(encoded).is_err());
        let blob = valid.to_large_blob().unwrap();
        assert_eq!(
            PasskeyAttestationResponse::from_large_blob(&blob).unwrap(),
            valid
        );
        assert!(valid.object().unwrap().authenticator_data().user_present());
        assert!(valid.object().unwrap().authenticator_data().user_verified());
    }
}
