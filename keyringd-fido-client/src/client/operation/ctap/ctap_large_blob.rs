use crate::{CtapClient, FidoClientError, PinUvAuthToken};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use keyringd_fido::ctap::AuthenticatorLargeBlobsResponse;
use keyringd_fido::ctap::AuthenticatorMakeCredentialResponse;
use keyringd_fido::webauthn::AuthenticationExtensionsAuthenticatorOutputs;
use keyringd_fido::{
    CtapCommand, CtapRequest, EncodedCborValue,
    ctap::{
        AuthenticatorGetAssertionRequest, AuthenticatorGetInfoResponse,
        AuthenticatorLargeBlobExtension, AuthenticatorLargeBlobOutput,
        AuthenticatorLargeBlobsRequest, AuthenticatorMakeCredentialRequest, LargeBlobArray,
    },
    webauthn::{
        AuthenticationExtensionsAuthenticatorInputs, LargeBlobInput, LargeBlobOutput,
        LargeBlobSupport,
    },
};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

const MAX_BLOB_SIZE: usize = 1024 * 1024;

/// Optional large-blob processing on the already authenticated CTAP connection.
/// Never starts another assertion or prompts for another authorization.
pub(super) struct CtapLargeBlob {
    input: Option<LargeBlobInput>,
    mode: LargeBlobMode,
}
#[derive(Clone, Copy)]
enum LargeBlobMode {
    Unsupported,
    LargeBlob { max_message: usize },
    LargeBlobKey { fragment: usize, capacity: usize },
}
impl CtapLargeBlob {
    pub(super) fn new(
        input: Option<&LargeBlobInput>,
        info: &AuthenticatorGetInfoResponse,
        max_message: usize,
    ) -> Self {
        let extensions = info.extensions();
        let has = |name: &str| extensions.is_some_and(|e| e.iter().any(|v| v == name));
        let mode = if has("largeBlob") {
            LargeBlobMode::LargeBlob { max_message }
        } else if has("largeBlobKey")
            && info
                .options()
                .and_then(|options| options.get("largeBlobs"))
                .copied()
                .unwrap_or(false)
        {
            LargeBlobMode::LargeBlobKey {
                fragment: max_message.saturating_sub(64).max(1),
                capacity: info
                    .max_serialized_large_blob_array()
                    .unwrap_or(0)
                    .min(MAX_BLOB_SIZE as u64) as usize,
            }
        } else {
            LargeBlobMode::Unsupported
        };
        Self {
            input: input.cloned(),
            mode,
        }
    }

    pub(super) fn needs_write_token(&self) -> bool {
        matches!(self.mode, LargeBlobMode::LargeBlobKey { .. })
            && self
                .input
                .as_ref()
                .is_some_and(|input| input.write().is_some())
    }
    fn extensions(
        &self,
        existing: Option<&AuthenticationExtensionsAuthenticatorInputs>,
        registration: bool,
    ) -> Result<Option<AuthenticationExtensionsAuthenticatorInputs>, FidoClientError> {
        let Some(input) = &self.input else {
            return Ok(None);
        };
        if !registration && input.read() != Some(true) && input.write().is_none() {
            return Ok(None);
        }
        let value = match self.mode {
            LargeBlobMode::LargeBlob { .. } => {
                let extension = if registration {
                    AuthenticatorLargeBlobExtension::Support {
                        required: input.support() == Some(&LargeBlobSupport::Required),
                    }
                } else if let Some(bytes) = input.write() {
                    AuthenticatorLargeBlobExtension::Write {
                        compressed: Self::compress(bytes)?,
                        original_size: bytes.len() as u64,
                    }
                } else {
                    AuthenticatorLargeBlobExtension::Read
                };
                Some((
                    "largeBlob",
                    extension
                        .to_encoded_value()
                        .map_err(FidoClientError::protocol)?,
                ))
            }
            LargeBlobMode::LargeBlobKey { .. } => Some((
                "largeBlobKey",
                EncodedCborValue::from_canonical_bytes(vec![0xf5])
                    .map_err(FidoClientError::protocol)?,
            )),
            LargeBlobMode::Unsupported => None,
        };
        if registration && input.support() == Some(&LargeBlobSupport::Required) && value.is_none() {
            return Err(FidoClientError::UnsupportedRequirement(
                "largeBlob is required".into(),
            ));
        }
        Ok(value.map(|(name, value)| {
            existing
                .cloned()
                .unwrap_or_default()
                .with_value(name, value)
        }))
    }
    pub(super) fn configure_make(
        &self,
        request: &mut AuthenticatorMakeCredentialRequest,
    ) -> Result<(), FidoClientError> {
        if let Some(ext) = self.extensions(request.extensions(), true)? {
            *request = request.clone().with_extensions(ext);
        }
        Ok(())
    }
    pub(super) fn configure_get(
        &self,
        request: &mut AuthenticatorGetAssertionRequest,
    ) -> Result<(), FidoClientError> {
        let ext = match self.extensions(request.extensions(), false) {
            Ok(ext) => ext,
            Err(_)
                if self
                    .input
                    .as_ref()
                    .is_some_and(|input| input.write().is_some()) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        if let Some(ext) = ext {
            let candidate = request.clone().with_extensions(ext);
            // Reserve room for the subsequently added PIN/UV authentication fields.
            if let LargeBlobMode::LargeBlob { max_message } = self.mode
                && self
                    .input
                    .as_ref()
                    .is_some_and(|input| input.write().is_some())
                && candidate
                    .to_cbor_bytes()
                    .map_err(FidoClientError::protocol)?
                    .len()
                    + 96
                    > max_message
            {
                return Ok(());
            }
            *request = candidate;
        }
        Ok(())
    }
    pub(super) fn registration_output(
        &self,
        response: &AuthenticatorMakeCredentialResponse,
    ) -> Option<LargeBlobOutput> {
        self.input.as_ref()?;
        let supported = match self.mode {
            LargeBlobMode::LargeBlobKey { .. } => response.large_blob_key().is_some(),
            LargeBlobMode::LargeBlob { .. } => matches!(
                Self::large_blob_extension_output(response.unsigned_extension_outputs()),
                Some(AuthenticatorLargeBlobOutput::Supported(true))
            ),
            LargeBlobMode::Unsupported => false,
        };
        Some(LargeBlobOutput::Supported(supported))
    }
    fn large_blob_extension_output(
        outputs: Option<&AuthenticationExtensionsAuthenticatorOutputs>,
    ) -> Option<AuthenticatorLargeBlobOutput> {
        AuthenticatorLargeBlobOutput::from_cbor_bytes(
            outputs?.values().get("largeBlob")?.as_bytes(),
        )
        .ok()
    }
    pub(super) async fn complete(
        &self,
        client: &mut CtapClient,
        responses: &mut [crate::AssertionResponse],
        token: Option<&PinUvAuthToken>,
    ) {
        // Read the shared array once even when multiple discoverable credentials are returned.
        let mut array = if matches!(self.mode, LargeBlobMode::LargeBlobKey { .. })
            && self
                .input
                .as_ref()
                .is_some_and(|input| input.read() == Some(true) || input.write().is_some())
        {
            self.read_array(client).await.ok()
        } else {
            None
        };
        for response in responses {
            let output = match self
                .input
                .as_ref()
                .map(|input| (input.read(), input.write()))
            {
                Some((Some(true), _)) => {
                    let blob = match self.mode {
                        LargeBlobMode::LargeBlob { .. } => {
                            match Self::large_blob_extension_output(
                                response.unsigned_extension_outputs(),
                            ) {
                                Some(AuthenticatorLargeBlobOutput::Read {
                                    compressed,
                                    original_size,
                                }) => Self::decompress(&compressed, original_size).ok(),
                                _ => None,
                            }
                        }
                        LargeBlobMode::LargeBlobKey { .. } => array.as_ref().and_then(|array| {
                            Self::find_blob(array, response.large_blob_key()?).map(|(_, blob)| blob)
                        }),
                        LargeBlobMode::Unsupported => None,
                    };
                    Some(LargeBlobOutput::Read(blob))
                }
                Some((_, Some(data))) => {
                    let written = match self.mode {
                        LargeBlobMode::LargeBlob { .. } => matches!(
                            Self::large_blob_extension_output(
                                response.unsigned_extension_outputs()
                            ),
                            Some(AuthenticatorLargeBlobOutput::Written(true))
                        ),
                        LargeBlobMode::LargeBlobKey { .. } => {
                            if let (Some(array), Some(key), Some(token)) =
                                (array.as_mut(), response.large_blob_key(), token)
                            {
                                self.write_blob(client, array, key, data, token)
                                    .await
                                    .is_ok()
                            } else {
                                false
                            }
                        }
                        LargeBlobMode::Unsupported => false,
                    };
                    Some(LargeBlobOutput::Written(written))
                }
                _ => None,
            };
            if let Some(output) = output {
                response.set_large_blob(output);
            }
        }
    }
    async fn read_array(&self, client: &mut CtapClient) -> Result<LargeBlobArray, FidoClientError> {
        let LargeBlobMode::LargeBlobKey {
            fragment: fragment_length,
            capacity,
        } = self.mode
        else {
            return Err(Self::invalid());
        };
        let mut bytes = Vec::new();
        loop {
            let request = AuthenticatorLargeBlobsRequest::Read {
                offset: bytes.len() as u64,
                length: fragment_length as u64,
            };
            let payload = client
                .execute(CtapRequest::new(
                    CtapCommand::LargeBlobs,
                    request.to_cbor_bytes().map_err(FidoClientError::protocol)?,
                ))
                .await?;
            let fragment = AuthenticatorLargeBlobsResponse::from_cbor_bytes(&payload)
                .map_err(FidoClientError::protocol)?
                .config()
                .to_vec();
            if fragment.len() > fragment_length || bytes.len() + fragment.len() > capacity {
                return Err(Self::invalid());
            }
            let done = fragment.len() < fragment_length;
            bytes.extend_from_slice(&fragment);
            if done {
                break;
            }
        }
        LargeBlobArray::from_bytes(&bytes).map_err(FidoClientError::protocol)
    }
    fn aad(size: u64) -> Vec<u8> {
        let mut aad = b"blob".to_vec();
        aad.extend_from_slice(&size.to_le_bytes());
        aad
    }
    fn find_blob(array: &LargeBlobArray, key: &[u8]) -> Option<(usize, Vec<u8>)> {
        let cipher = Aes256Gcm::new_from_slice(key).ok()?;
        for i in 0..array.len() {
            let Some(entry) = array.entry(i) else {
                continue;
            };
            let (data, nonce, size) = (entry.ciphertext, entry.nonce, entry.orig_size);
            if let Ok(plaintext) = cipher.decrypt(
                &Nonce::from(*nonce),
                Payload {
                    msg: data,
                    aad: &Self::aad(size),
                },
            ) {
                return Self::decompress(&plaintext, size)
                    .ok()
                    .map(|data| (i, data));
            }
        }
        None
    }
    async fn write_blob(
        &self,
        client: &mut CtapClient,
        array: &mut LargeBlobArray,
        key: &[u8],
        data: &[u8],
        token: &PinUvAuthToken,
    ) -> Result<(), FidoClientError> {
        let LargeBlobMode::LargeBlobKey {
            fragment: fragment_length,
            capacity,
        } = self.mode
        else {
            return Err(Self::invalid());
        };
        let compressed = Self::compress(data)?;
        let mut nonce = [0; 12];
        OsRng.fill_bytes(&mut nonce);
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| Self::invalid())?;
        let ciphertext = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &compressed,
                    aad: &Self::aad(data.len() as u64),
                },
            )
            .map_err(|_| Self::invalid())?;
        let index = Self::find_blob(array, key).map(|(index, _)| index);
        array.replace(index, ciphertext, nonce, data.len() as u64);
        let bytes = array.to_bytes().map_err(FidoClientError::protocol)?;
        if bytes.len() > capacity {
            return Err(Self::invalid());
        }
        for (i, fragment) in bytes.chunks(fragment_length).enumerate() {
            let offset = i * fragment_length;
            let mut message = vec![0xff; 32];
            message.extend_from_slice(&[0x0c, 0x00]);
            message.extend_from_slice(&(offset as u32).to_le_bytes());
            message.extend_from_slice(&Sha256::digest(fragment));
            let auth = token
                .authenticate(&message)
                .map_err(FidoClientError::protocol)?;
            let request = AuthenticatorLargeBlobsRequest::Write {
                offset: offset as u64,
                data: fragment.to_vec(),
                total_length: (offset == 0).then_some(bytes.len() as u64),
                pin_uv_auth: Some((auth, token.protocol().version())),
            };
            client
                .execute(CtapRequest::new(
                    CtapCommand::LargeBlobs,
                    request.to_cbor_bytes().map_err(FidoClientError::protocol)?,
                ))
                .await?;
        }
        Ok(())
    }
    fn compress(data: &[u8]) -> Result<Vec<u8>, FidoClientError> {
        if data.len() > MAX_BLOB_SIZE {
            return Err(Self::invalid());
        }
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).map_err(FidoClientError::protocol)?;
        encoder.finish().map_err(FidoClientError::protocol)
    }
    fn decompress(data: &[u8], size: u64) -> Result<Vec<u8>, FidoClientError> {
        if size > MAX_BLOB_SIZE as u64 {
            return Err(Self::invalid());
        }
        let mut result = Vec::new();
        DeflateDecoder::new(data)
            .take(size + 1)
            .read_to_end(&mut result)
            .map_err(FidoClientError::protocol)?;
        if result.len() as u64 != size {
            return Err(Self::invalid());
        }
        Ok(result)
    }
    fn invalid() -> FidoClientError {
        FidoClientError::UnsupportedRequirement(
            "largeBlob data is invalid or exceeds storage limits".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockCtapTransport;
    use keyringd_fido::ctap::AuthenticatorLargeBlobsResponse;
    use keyringd_fido::{CtapResponse, CtapStatusCode};
    use std::sync::{Arc, Mutex};

    #[test]
    fn empty_inputs_request_registration_support_but_do_not_start_authentication_processing() {
        let options = keyringd_fido::webauthn::PublicKeyCredentialRequestOptions::from_json_bytes(
            br#"{"challenge":"AQ","extensions":{"largeBlob":{"read":false}}}"#,
        )
        .unwrap();
        let disabled_input = options.extensions().unwrap().large_blob().unwrap();
        for mode in [
            LargeBlobMode::LargeBlob { max_message: 1024 },
            LargeBlobMode::LargeBlobKey {
                fragment: 960,
                capacity: 4096,
            },
        ] {
            let processor = CtapLargeBlob {
                input: Some(LargeBlobInput::default()),
                mode,
            };
            assert!(processor.extensions(None, true).unwrap().is_some());
            assert!(processor.extensions(None, false).unwrap().is_none());
            let disabled = CtapLargeBlob {
                input: Some(disabled_input.clone()),
                mode,
            };
            assert!(disabled.extensions(None, false).unwrap().is_none());
            assert!(!disabled.needs_write_token());
        }
        let required = CtapLargeBlob {
            input: Some(LargeBlobInput::request_support(LargeBlobSupport::Required)),
            mode: LargeBlobMode::Unsupported,
        };
        assert!(required.extensions(None, true).is_err());
    }

    #[test]
    fn trial_decryption_skips_an_unrelated_malformed_entry() {
        let key = [3; 32];
        let data = b"saved registration";
        let nonce = [4; 12];
        let ciphertext = Aes256Gcm::new_from_slice(&key)
            .unwrap()
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &CtapLargeBlob::compress(data).unwrap(),
                    aad: &CtapLargeBlob::aad(data.len() as u64),
                },
            )
            .unwrap();
        let mut array = LargeBlobArray::default();
        array.replace(None, ciphertext, nonce, data.len() as u64);
        let bytes = array.to_bytes().unwrap();
        // Prepend a valid CBOR integer that is not a conforming large-blob map.
        let mut mixed = vec![0x82, 0];
        mixed.extend_from_slice(&bytes[1..bytes.len() - 16]);
        let checksum = Sha256::digest(&mixed);
        mixed.extend_from_slice(&checksum[..16]);
        let array = LargeBlobArray::from_bytes(&mixed).unwrap();
        assert_eq!(
            CtapLargeBlob::find_blob(&array, &key),
            Some((1, data.to_vec()))
        );
    }

    #[tokio::test]
    async fn encrypted_array_round_trip_preserves_other_credentials_and_authenticates_fragments() {
        let token = crate::PinUvAuthToken::new(crate::PinUvAuthProtocol::Two, vec![7; 32]).unwrap();
        let storage = Arc::new(Mutex::new(LargeBlobArray::default().to_bytes().unwrap()));
        let mut transport = MockCtapTransport::new();
        let shared = storage.clone();
        let expected_token =
            crate::PinUvAuthToken::new(crate::PinUvAuthProtocol::Two, vec![7; 32]).unwrap();
        transport.expect_transmit().returning(move |request| {
            assert_eq!(request.command(), CtapCommand::LargeBlobs);
            let mut storage = shared.lock().unwrap();
            let payload =
                match AuthenticatorLargeBlobsRequest::from_cbor_bytes(request.payload()).unwrap() {
                    AuthenticatorLargeBlobsRequest::Read { offset, length } => {
                        let start = (offset as usize).min(storage.len());
                        let end = (start + length as usize).min(storage.len());
                        AuthenticatorLargeBlobsResponse::new(storage[start..end].to_vec())
                            .to_cbor_bytes()
                            .unwrap()
                    }
                    AuthenticatorLargeBlobsRequest::Write {
                        offset,
                        data,
                        total_length,
                        pin_uv_auth,
                    } => {
                        let (auth, protocol) = pin_uv_auth.unwrap();
                        assert_eq!(protocol, 2);
                        let mut message = vec![0xff; 32];
                        message.extend_from_slice(&[0x0c, 0]);
                        message.extend_from_slice(&(offset as u32).to_le_bytes());
                        message.extend_from_slice(&Sha256::digest(&data));
                        assert_eq!(auth, expected_token.authenticate(&message).unwrap());
                        if let Some(length) = total_length {
                            storage.clear();
                            storage.reserve(length as usize);
                        }
                        assert_eq!(offset as usize, storage.len());
                        storage.extend_from_slice(&data);
                        Vec::new()
                    }
                };
            Box::pin(async move { Ok(CtapResponse::new(CtapStatusCode::Ok, payload)) })
        });
        let mut client = CtapClient::new(Box::new(transport));
        let processor = CtapLargeBlob {
            input: None,
            mode: LargeBlobMode::LargeBlobKey {
                fragment: 32,
                capacity: 4096,
            },
        };
        let mut array = processor.read_array(&mut client).await.unwrap();
        processor
            .write_blob(&mut client, &mut array, &[1; 32], b"credential one", &token)
            .await
            .unwrap();
        processor
            .write_blob(&mut client, &mut array, &[2; 32], b"credential two", &token)
            .await
            .unwrap();
        let mut restored = processor.read_array(&mut client).await.unwrap();
        assert_eq!(
            CtapLargeBlob::find_blob(&restored, &[1; 32]).unwrap().1,
            b"credential one"
        );
        assert_eq!(
            CtapLargeBlob::find_blob(&restored, &[2; 32]).unwrap().1,
            b"credential two"
        );
        assert!(CtapLargeBlob::find_blob(&restored, &[3; 32]).is_none());
        processor
            .write_blob(&mut client, &mut restored, &[1; 32], b"updated", &token)
            .await
            .unwrap();
        let restored = processor.read_array(&mut client).await.unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(
            CtapLargeBlob::find_blob(&restored, &[1; 32]).unwrap().1,
            b"updated"
        );
        assert_eq!(
            CtapLargeBlob::find_blob(&restored, &[2; 32]).unwrap().1,
            b"credential two"
        );
    }
    #[test]
    fn rejects_decompression_bombs_and_mismatched_original_sizes() {
        let data = CtapLargeBlob::compress(b"blob").unwrap();
        assert!(CtapLargeBlob::decompress(&data, MAX_BLOB_SIZE as u64 + 1).is_err());
        assert!(CtapLargeBlob::decompress(&data, 3).is_err());
        assert!(CtapLargeBlob::decompress(&data, 5).is_err());
    }
}
