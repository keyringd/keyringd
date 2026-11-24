use super::{AttestationObject, AuthenticatorTransport, CoseAlgorithmIdentifier, ProtocolError};
use crate::codec::JsonObject;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorAttestationResponse {
    client_data_json: Vec<u8>,
    attestation_object: Vec<u8>,
    transports: Vec<AuthenticatorTransport>,
}

impl AuthenticatorAttestationResponse {
    pub fn from_attestation_object(
        client_data_json: Vec<u8>,
        attestation_object: AttestationObject,
        transports: Vec<AuthenticatorTransport>,
    ) -> Result<Self, ProtocolError> {
        validate_transports(&transports)?;
        Ok(Self {
            client_data_json,
            attestation_object: attestation_object.to_cbor_bytes()?,
            transports,
        })
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_json_value(JsonObject::from_bytes(bytes)?.into_value())
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        JsonObject::from_value(self.to_json_value()?)?.into_bytes()
    }

    pub(crate) fn from_json_value(value: serde_json::Value) -> Result<Self, ProtocolError> {
        let mut fields = JsonObject::from_value(value)?;
        let response = Self {
            client_data_json: fields.take_binary("clientDataJSON")?,
            attestation_object: fields.take_binary("attestationObject")?,
            transports: fields.take("transports")?,
        };
        validate_transports(&response.transports)?;
        let encoded_authenticator_data = fields.take_binary("authenticatorData")?;
        let encoded_public_key = fields.take_optional_binary("publicKey")?;
        let encoded_algorithm = CoseAlgorithmIdentifier::new(fields.take("publicKeyAlgorithm")?);
        if response.authenticator_data()?.to_bytes()? != encoded_authenticator_data
            || response.public_key()? != encoded_public_key
            || response.public_key_algorithm()? != encoded_algorithm
        {
            return Err(ProtocolError::UnexpectedType {
                field: "AuthenticatorAttestationResponseJSON derived members",
                expected: "values derived from attestationObject",
            });
        }
        Ok(response)
    }

    pub(crate) fn to_json_value(&self) -> Result<serde_json::Value, ProtocolError> {
        let mut fields = JsonObject::default();
        fields.put_binary("clientDataJSON", self.client_data_json());
        let authenticator_data = self.authenticator_data()?.to_bytes()?;
        let public_key = self.public_key()?;
        fields.put_binary("authenticatorData", &authenticator_data);
        fields.put("transports", self.transports())?;
        fields.put_optional_binary("publicKey", public_key.as_deref());
        fields.put("publicKeyAlgorithm", &self.public_key_algorithm()?.value())?;
        fields.put_binary("attestationObject", self.attestation_object());
        Ok(fields.into_value())
    }

    pub fn client_data_json(&self) -> &[u8] {
        &self.client_data_json
    }

    pub fn attestation_object(&self) -> &[u8] {
        &self.attestation_object
    }

    pub fn transports(&self) -> &[AuthenticatorTransport] {
        &self.transports
    }

    pub fn authenticator_data(&self) -> Result<super::AuthenticatorData, ProtocolError> {
        Ok(
            AttestationObject::from_cbor_bytes(&self.attestation_object)?
                .authenticator_data()
                .clone(),
        )
    }

    /// Returns DER SubjectPublicKeyInfo, or None when the key cannot be converted.
    pub fn public_key(&self) -> Result<Option<Vec<u8>>, ProtocolError> {
        let authenticator_data = self.authenticator_data()?;
        match authenticator_data.attested_credential_data() {
            Some(attested) => attested.credential_public_key().to_spki_der(),
            None => Ok(None),
        }
    }

    pub fn public_key_algorithm(&self) -> Result<CoseAlgorithmIdentifier, ProtocolError> {
        self.authenticator_data()?
            .attested_credential_data()
            .map(|attested| attested.credential_public_key().algorithm())
            .ok_or(ProtocolError::MissingField("attestedCredentialData"))
    }
}

fn validate_transports(transports: &[AuthenticatorTransport]) -> Result<(), ProtocolError> {
    if transports
        .windows(2)
        .any(|pair| pair[0].as_str() >= pair[1].as_str())
    {
        return Err(ProtocolError::UnexpectedType {
            field: "AuthenticatorAttestationResponse.transports",
            expected: "unique transport identifiers in lexicographical order",
        });
    }
    Ok(())
}
