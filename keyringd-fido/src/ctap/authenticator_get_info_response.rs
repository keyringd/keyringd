use std::collections::{BTreeMap, HashSet};

use crate::codec::CborItem as Value;

use super::{CtapValue, CtapVersion, ProtocolError, PublicKeyCredentialParameters};
use crate::AuthenticatorTransport;
use crate::codec::CborCodec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorGetInfoResponse {
    versions: Vec<String>,
    extensions: Option<Vec<String>>,
    aaguid: [u8; 16],
    options: Option<BTreeMap<String, bool>>,
    max_message_size: Option<u64>,
    pin_uv_auth_protocols: Option<Vec<u64>>,
    max_credential_count_in_list: Option<u64>,
    max_credential_id_length: Option<u64>,
    transports: Option<Vec<AuthenticatorTransport>>,
    algorithms: Option<Vec<PublicKeyCredentialParameters>>,
    max_serialized_large_blob_array: Option<u64>,
    force_pin_change: Option<bool>,
    pub(crate) min_pin_length: Option<u64>,
    firmware_version: Option<u64>,
    max_credential_blob_length: Option<u64>,
    max_rp_ids_for_set_min_pin_length: Option<u64>,
    preferred_platform_uv_attempts: Option<u64>,
    uv_modality: Option<u64>,
    certifications: Option<BTreeMap<String, u64>>,
    remaining_discoverable_credentials: Option<u64>,
    vendor_prototype_config_commands: Option<Vec<u64>>,
    attestation_formats: Option<Vec<String>>,
    uv_count_since_last_pin_entry: Option<u64>,
    long_touch_for_reset: Option<bool>,
    encrypted_identifier: Option<Vec<u8>>,
    transports_for_reset: Option<Vec<AuthenticatorTransport>>,
    pin_complexity_policy: Option<bool>,
    pin_complexity_policy_url: Option<Vec<u8>>,
    max_pin_length: Option<u64>,
}

impl AuthenticatorGetInfoResponse {
    pub fn new(versions: Vec<String>, aaguid: [u8; 16]) -> Self {
        Self {
            versions,
            extensions: None,
            aaguid,
            options: None,
            max_message_size: None,
            pin_uv_auth_protocols: None,
            max_credential_count_in_list: None,
            max_credential_id_length: None,
            transports: None,
            algorithms: None,
            max_serialized_large_blob_array: None,
            force_pin_change: None,
            min_pin_length: None,
            firmware_version: None,
            max_credential_blob_length: None,
            max_rp_ids_for_set_min_pin_length: None,
            preferred_platform_uv_attempts: None,
            uv_modality: None,
            certifications: None,
            remaining_discoverable_credentials: None,
            vendor_prototype_config_commands: None,
            attestation_formats: None,
            uv_count_since_last_pin_entry: None,
            long_touch_for_reset: None,
            encrypted_identifier: None,
            transports_for_reset: None,
            pin_complexity_policy: None,
            pin_complexity_policy_url: None,
            max_pin_length: None,
        }
    }
    pub fn with_extensions(mut self, value: Vec<String>) -> Self {
        self.extensions = Some(value);
        self
    }
    pub fn with_options(mut self, value: BTreeMap<String, bool>) -> Self {
        self.options = Some(value);
        self
    }
    pub fn with_max_message_size(mut self, value: u64) -> Self {
        self.max_message_size = Some(value);
        self
    }
    pub fn with_pin_uv_auth_protocols(mut self, value: Vec<u64>) -> Self {
        self.pin_uv_auth_protocols = Some(value);
        self
    }
    pub fn with_max_credential_count_in_list(mut self, value: u64) -> Self {
        self.max_credential_count_in_list = Some(value);
        self
    }
    pub fn with_max_credential_id_length(mut self, value: u64) -> Self {
        self.max_credential_id_length = Some(value);
        self
    }
    pub fn with_transports(mut self, value: Vec<AuthenticatorTransport>) -> Self {
        self.transports = Some(value);
        self
    }
    pub fn with_algorithms(mut self, value: Vec<PublicKeyCredentialParameters>) -> Self {
        self.algorithms = Some(value);
        self
    }
    pub fn with_max_serialized_large_blob_array(mut self, value: u64) -> Self {
        self.max_serialized_large_blob_array = Some(value);
        self
    }
    pub fn with_force_pin_change(mut self, value: bool) -> Self {
        self.force_pin_change = Some(value);
        self
    }
    pub fn with_min_pin_length(mut self, value: u64) -> Self {
        self.min_pin_length = Some(value);
        self
    }
    pub fn with_firmware_version(mut self, value: u64) -> Self {
        self.firmware_version = Some(value);
        self
    }
    pub fn with_max_credential_blob_length(mut self, value: u64) -> Self {
        self.max_credential_blob_length = Some(value);
        self
    }
    pub fn with_max_rp_ids_for_set_min_pin_length(mut self, value: u64) -> Self {
        self.max_rp_ids_for_set_min_pin_length = Some(value);
        self
    }
    pub fn with_preferred_platform_uv_attempts(mut self, value: u64) -> Self {
        self.preferred_platform_uv_attempts = Some(value);
        self
    }
    pub fn with_uv_modality(mut self, value: u64) -> Self {
        self.uv_modality = Some(value);
        self
    }
    pub fn with_certifications(mut self, value: BTreeMap<String, u64>) -> Self {
        self.certifications = Some(value);
        self
    }
    pub fn with_remaining_discoverable_credentials(mut self, value: u64) -> Self {
        self.remaining_discoverable_credentials = Some(value);
        self
    }
    pub fn with_vendor_prototype_config_commands(mut self, value: Vec<u64>) -> Self {
        self.vendor_prototype_config_commands = Some(value);
        self
    }
    pub fn with_attestation_formats(mut self, value: Vec<String>) -> Self {
        self.attestation_formats = Some(value);
        self
    }
    pub fn with_uv_count_since_last_pin_entry(mut self, value: u64) -> Self {
        self.uv_count_since_last_pin_entry = Some(value);
        self
    }
    pub fn with_long_touch_for_reset(mut self, value: bool) -> Self {
        self.long_touch_for_reset = Some(value);
        self
    }
    pub fn with_encrypted_identifier(mut self, value: Vec<u8>) -> Self {
        self.encrypted_identifier = Some(value);
        self
    }
    pub fn with_transports_for_reset(mut self, value: Vec<AuthenticatorTransport>) -> Self {
        self.transports_for_reset = Some(value);
        self
    }
    pub fn with_pin_complexity_policy(mut self, value: bool) -> Self {
        self.pin_complexity_policy = Some(value);
        self
    }
    pub fn with_pin_complexity_policy_url(mut self, value: Vec<u8>) -> Self {
        self.pin_complexity_policy_url = Some(value);
        self
    }
    pub fn with_max_pin_length(mut self, value: u64) -> Self {
        self.max_pin_length = Some(value);
        self
    }
    pub fn versions(&self) -> &[String] {
        &self.versions
    }
    pub fn highest_supported_version(&self) -> Option<CtapVersion> {
        CtapVersion::highest_supported(self.versions.iter().map(String::as_str))
    }
    pub fn extensions(&self) -> Option<&[String]> {
        self.extensions.as_deref()
    }
    pub const fn aaguid(&self) -> &[u8; 16] {
        &self.aaguid
    }
    pub fn options(&self) -> Option<&BTreeMap<String, bool>> {
        self.options.as_ref()
    }
    pub const fn max_message_size(&self) -> Option<u64> {
        self.max_message_size
    }
    pub fn pin_uv_auth_protocols(&self) -> Option<&[u64]> {
        self.pin_uv_auth_protocols.as_deref()
    }
    pub const fn max_credential_count_in_list(&self) -> Option<u64> {
        self.max_credential_count_in_list
    }
    pub const fn max_credential_id_length(&self) -> Option<u64> {
        self.max_credential_id_length
    }
    pub fn transports(&self) -> Option<&[AuthenticatorTransport]> {
        self.transports.as_deref()
    }
    pub fn algorithms(&self) -> Option<&[PublicKeyCredentialParameters]> {
        self.algorithms.as_deref()
    }
    pub const fn max_serialized_large_blob_array(&self) -> Option<u64> {
        self.max_serialized_large_blob_array
    }
    pub const fn force_pin_change(&self) -> Option<bool> {
        self.force_pin_change
    }
    pub const fn min_pin_length(&self) -> Option<u64> {
        self.min_pin_length
    }
    pub const fn firmware_version(&self) -> Option<u64> {
        self.firmware_version
    }
    pub const fn max_credential_blob_length(&self) -> Option<u64> {
        self.max_credential_blob_length
    }
    pub const fn max_rp_ids_for_set_min_pin_length(&self) -> Option<u64> {
        self.max_rp_ids_for_set_min_pin_length
    }
    pub const fn preferred_platform_uv_attempts(&self) -> Option<u64> {
        self.preferred_platform_uv_attempts
    }
    pub const fn uv_modality(&self) -> Option<u64> {
        self.uv_modality
    }
    pub fn certifications(&self) -> Option<&BTreeMap<String, u64>> {
        self.certifications.as_ref()
    }
    pub const fn remaining_discoverable_credentials(&self) -> Option<u64> {
        self.remaining_discoverable_credentials
    }
    pub fn vendor_prototype_config_commands(&self) -> Option<&[u64]> {
        self.vendor_prototype_config_commands.as_deref()
    }
    pub fn attestation_formats(&self) -> Option<&[String]> {
        self.attestation_formats.as_deref()
    }
    pub const fn uv_count_since_last_pin_entry(&self) -> Option<u64> {
        self.uv_count_since_last_pin_entry
    }
    pub const fn long_touch_for_reset(&self) -> Option<bool> {
        self.long_touch_for_reset
    }
    pub fn encrypted_identifier(&self) -> Option<&[u8]> {
        self.encrypted_identifier.as_deref()
    }
    pub fn transports_for_reset(&self) -> Option<&[AuthenticatorTransport]> {
        self.transports_for_reset.as_deref()
    }
    pub const fn pin_complexity_policy(&self) -> Option<bool> {
        self.pin_complexity_policy
    }
    pub fn pin_complexity_policy_url(&self) -> Option<&[u8]> {
        self.pin_complexity_policy_url.as_deref()
    }
    pub const fn max_pin_length(&self) -> Option<u64> {
        self.max_pin_length
    }
    pub fn from_cbor_bytes(encoded: &[u8]) -> Result<Self, ProtocolError> {
        let mut fields = CtapValue::ctap_map(CborCodec::decode(encoded)?)?;
        let versions = CtapValue::text_array(
            fields
                .remove(&0x01)
                .ok_or(ProtocolError::MissingCtapField(0x01))?,
            "versions",
            false,
        )?;
        let aaguid = CtapValue::bytes(
            fields
                .remove(&0x03)
                .ok_or(ProtocolError::MissingCtapField(0x03))?,
            "aaguid",
        )?;
        if aaguid.len() != 16 {
            return Err(ProtocolError::InvalidLength {
                field: "aaguid",
                expected: 16,
                actual: aaguid.len(),
            });
        }

        let response = Self {
            versions,
            extensions: fields
                .remove(&0x02)
                .map(|value| CtapValue::text_array(value, "extensions", true))
                .transpose()?,
            aaguid: aaguid.try_into().expect("validated AAGUID length"),
            options: fields
                .remove(&0x04)
                .map(CtapValue::boolean_map)
                .transpose()?,
            max_message_size: take_unsigned(&mut fields, 0x05, "maxMsgSize")?,
            pin_uv_auth_protocols: fields
                .remove(&0x06)
                .map(|value| CtapValue::unsigned_array(value, "pinUvAuthProtocols", false))
                .transpose()?,
            max_credential_count_in_list: take_positive(
                &mut fields,
                0x07,
                "maxCredentialCountInList",
            )?,
            max_credential_id_length: take_positive(&mut fields, 0x08, "maxCredentialIdLength")?,
            transports: fields
                .remove(&0x09)
                .map(|value| transport_array(value, "transports"))
                .transpose()?,
            algorithms: fields
                .remove(&0x0a)
                .map(PublicKeyCredentialParameters::vec_from_value)
                .transpose()?,
            max_serialized_large_blob_array: take_unsigned(
                &mut fields,
                0x0b,
                "maxSerializedLargeBlobArray",
            )?,
            force_pin_change: take_bool(&mut fields, 0x0c, "forcePINChange")?,
            min_pin_length: take_unsigned(&mut fields, 0x0d, "minPINLength")?,
            firmware_version: take_unsigned(&mut fields, 0x0e, "firmwareVersion")?,
            max_credential_blob_length: take_unsigned(&mut fields, 0x0f, "maxCredBlobLength")?,
            max_rp_ids_for_set_min_pin_length: take_unsigned(
                &mut fields,
                0x10,
                "maxRPIDsForSetMinPINLength",
            )?,
            preferred_platform_uv_attempts: take_positive(
                &mut fields,
                0x11,
                "preferredPlatformUvAttempts",
            )?,
            uv_modality: take_unsigned(&mut fields, 0x12, "uvModality")?,
            certifications: fields
                .remove(&0x13)
                .map(CtapValue::unsigned_map)
                .transpose()?,
            remaining_discoverable_credentials: take_unsigned(
                &mut fields,
                0x14,
                "remainingDiscoverableCredentials",
            )?,
            vendor_prototype_config_commands: fields
                .remove(&0x15)
                .map(|value| {
                    CtapValue::unsigned_array(value, "vendorPrototypeConfigCommands", true)
                })
                .transpose()?,
            attestation_formats: fields
                .remove(&0x16)
                .map(|value| CtapValue::text_array(value, "attestationFormats", false))
                .transpose()?,
            uv_count_since_last_pin_entry: take_unsigned(
                &mut fields,
                0x17,
                "uvCountSinceLastPinEntry",
            )?,
            long_touch_for_reset: take_bool(&mut fields, 0x18, "longTouchForReset")?,
            encrypted_identifier: take_bytes(&mut fields, 0x19, "encIdentifier")?,
            transports_for_reset: fields
                .remove(&0x1a)
                .map(|value| transport_array(value, "transportsForReset"))
                .transpose()?,
            pin_complexity_policy: take_bool(&mut fields, 0x1b, "pinComplexityPolicy")?,
            pin_complexity_policy_url: take_bytes(&mut fields, 0x1c, "pinComplexityPolicyURL")?,
            max_pin_length: take_unsigned(&mut fields, 0x1d, "maxPINLength")?,
        };
        response.validate_cross_fields()?;
        Ok(response)
    }

    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate_cross_fields()?;
        let mut fields = Vec::new();
        fields.push((1u64.into(), text_values(&self.versions)));
        if let Some(value) = &self.extensions {
            fields.push((2u64.into(), text_values(value)));
        }
        fields.push((3u64.into(), Value::ByteString(self.aaguid.to_vec())));
        if let Some(value) = &self.options {
            fields.push((4u64.into(), bool_map(value)));
        }
        push_unsigned(&mut fields, 0x05, self.max_message_size);
        if let Some(value) = &self.pin_uv_auth_protocols {
            fields.push((6u64.into(), unsigned_values(value)));
        }
        push_unsigned(&mut fields, 0x07, self.max_credential_count_in_list);
        push_unsigned(&mut fields, 0x08, self.max_credential_id_length);
        if let Some(value) = &self.transports {
            fields.push((9u64.into(), transport_values(value)));
        }
        if let Some(value) = &self.algorithms {
            fields.push((
                10u64.into(),
                PublicKeyCredentialParameters::vec_to_value(value)?,
            ));
        }
        push_unsigned(&mut fields, 0x0b, self.max_serialized_large_blob_array);
        push_bool(&mut fields, 0x0c, self.force_pin_change);
        push_unsigned(&mut fields, 0x0d, self.min_pin_length);
        push_unsigned(&mut fields, 0x0e, self.firmware_version);
        push_unsigned(&mut fields, 0x0f, self.max_credential_blob_length);
        push_unsigned(&mut fields, 0x10, self.max_rp_ids_for_set_min_pin_length);
        push_unsigned(&mut fields, 0x11, self.preferred_platform_uv_attempts);
        push_unsigned(&mut fields, 0x12, self.uv_modality);
        if let Some(value) = &self.certifications {
            fields.push((19u64.into(), unsigned_map(value)));
        }
        push_unsigned(&mut fields, 0x14, self.remaining_discoverable_credentials);
        if let Some(value) = &self.vendor_prototype_config_commands {
            fields.push((21u64.into(), unsigned_values(value)));
        }
        if let Some(value) = &self.attestation_formats {
            fields.push((22u64.into(), text_values(value)));
        }
        push_unsigned(&mut fields, 0x17, self.uv_count_since_last_pin_entry);
        push_bool(&mut fields, 0x18, self.long_touch_for_reset);
        push_bytes(&mut fields, 0x19, self.encrypted_identifier.as_deref());
        if let Some(value) = &self.transports_for_reset {
            fields.push((26u64.into(), transport_values(value)));
        }
        push_bool(&mut fields, 0x1b, self.pin_complexity_policy);
        push_bytes(&mut fields, 0x1c, self.pin_complexity_policy_url.as_deref());
        push_unsigned(&mut fields, 0x1d, self.max_pin_length);
        Ok(CborCodec::encode(Value::Map(fields))?)
    }

    pub fn supports_discoverable_credentials(&self) -> bool {
        self.options
            .as_ref()
            .and_then(|options| options.get("rk"))
            .copied()
            .unwrap_or(false)
    }

    fn validate_cross_fields(&self) -> Result<(), ProtocolError> {
        if self.versions.is_empty() {
            return Err(ProtocolError::EmptyValue { field: "versions" });
        }
        require_unique(&self.versions, "versions")?;
        if let Some(extensions) = &self.extensions {
            require_unique(extensions, "extensions")?;
        }
        if let Some(protocols) = &self.pin_uv_auth_protocols {
            require_non_empty_unique(protocols, "pinUvAuthProtocols")?;
        }
        let highest_supported_version = self.highest_supported_version();
        let is_fido_2_3 = highest_supported_version == Some(CtapVersion::Fido23);
        if is_fido_2_3
            && self
                .pin_uv_auth_protocols
                .as_ref()
                .is_some_and(|protocols| !protocols.contains(&2))
        {
            return Err(ProtocolError::InvalidValue {
                field: "pinUvAuthProtocols",
                expected: "protocol 2 whenever a FIDO_2_3 authenticator advertises any protocol",
            });
        }
        if let Some(transports) = &self.transports {
            require_non_empty_unique(transports, "transports")?;
        }
        if let Some(transports) = &self.transports_for_reset {
            require_non_empty_unique(transports, "transportsForReset")?;
        }
        if let Some(algorithms) = &self.algorithms {
            if algorithms.is_empty() {
                return Err(ProtocolError::EmptyValue {
                    field: "algorithms",
                });
            }
            let mut unique = HashSet::with_capacity(algorithms.len());
            if !algorithms.iter().all(|algorithm| {
                unique.insert((algorithm.credential_type().as_str(), algorithm.algorithm()))
            }) {
                return Err(ProtocolError::DuplicateValue {
                    field: "algorithms",
                });
            }
        }
        if let Some(formats) = &self.attestation_formats {
            require_non_empty_unique(formats, "attestationFormats")?;
            if formats.iter().any(|format| format == "none") {
                return Err(ProtocolError::UnexpectedType {
                    field: "attestationFormats",
                    expected: "registered formats other than the implicitly supported none format",
                });
            }
        }
        require_positive(
            self.max_credential_count_in_list,
            "maxCredentialCountInList",
        )?;
        require_positive(self.max_credential_id_length, "maxCredentialIdLength")?;
        require_positive(
            self.preferred_platform_uv_attempts,
            "preferredPlatformUvAttempts",
        )?;
        if self
            .max_serialized_large_blob_array
            .is_some_and(|value| value < 1024)
        {
            return Err(ProtocolError::UnexpectedType {
                field: "maxSerializedLargeBlobArray",
                expected: "an unsigned integer greater than or equal to 1024",
            });
        }
        if self
            .max_credential_blob_length
            .is_some_and(|value| value < 32)
        {
            return Err(ProtocolError::UnexpectedType {
                field: "maxCredBlobLength",
                expected: "an unsigned integer greater than or equal to 32",
            });
        }
        if self.max_pin_length.is_some_and(|value| value < 8) {
            return Err(ProtocolError::UnexpectedType {
                field: "maxPINLength",
                expected: "an unsigned integer greater than or equal to 8",
            });
        }
        if self.min_pin_length.is_some_and(|value| value < 4) {
            return Err(ProtocolError::UnexpectedType {
                field: "minPINLength",
                expected: "an unsigned integer greater than or equal to 4",
            });
        }
        if let Some(identifier) = &self.encrypted_identifier
            && identifier.len() != 32
        {
            return Err(ProtocolError::InvalidLength {
                field: "encIdentifier",
                expected: 32,
                actual: identifier.len(),
            });
        }
        let supports_client_pin = self
            .options
            .as_ref()
            .is_some_and(|options| options.contains_key("clientPin"));
        let requires_min_pin_length = matches!(
            highest_supported_version,
            Some(CtapVersion::Fido21 | CtapVersion::Fido23)
        );
        if requires_min_pin_length && supports_client_pin != self.min_pin_length.is_some() {
            return Err(ProtocolError::UnexpectedType {
                field: "minPINLength",
                expected: "present if and only if the clientPin option is present",
            });
        }
        if !supports_client_pin && self.max_pin_length.is_some() {
            return Err(ProtocolError::UnexpectedType {
                field: "maxPINLength",
                expected: "absent when the clientPin option is not supported",
            });
        }
        if supports_client_pin
            && self
                .uv_modality
                .is_some_and(|modality| modality & 0x0000_0800 != 0)
        {
            return Err(ProtocolError::InvalidValue {
                field: "uvModality",
                expected: "built-in verification methods without client PIN/passcode_external",
            });
        }
        if is_fido_2_3 {
            let option = |name| {
                self.options
                    .as_ref()
                    .and_then(|options| options.get(name))
                    .copied()
            };
            if option("rk") == Some(true) && option("clientPin").is_none() && option("uv").is_none()
            {
                return Err(ProtocolError::InvalidValue {
                    field: "options.clientPin/options.uv",
                    expected: "clientPin or built-in uv support when a FIDO_2_3 authenticator supports rk",
                });
            }
            if (option("clientPin") == Some(true) || option("uv") == Some(true))
                && option("pinUvAuthToken") != Some(true)
            {
                return Err(ProtocolError::InvalidValue {
                    field: "options.pinUvAuthToken",
                    expected: "true when a FIDO_2_3 authenticator has clientPin or uv enabled",
                });
            }
            if !self
                .extensions
                .as_ref()
                .is_some_and(|extensions| extensions.iter().any(|value| value == "hmac-secret"))
            {
                return Err(ProtocolError::InvalidValue {
                    field: "extensions.hmac-secret",
                    expected: "advertised by every FIDO_2_3 authenticator",
                });
            }
        }
        let supports_set_min_pin_length = self
            .options
            .as_ref()
            .and_then(|options| options.get("setMinPINLength"))
            .copied()
            .unwrap_or(false);
        let advertises_set_min_pin_length = self
            .options
            .as_ref()
            .is_some_and(|options| options.contains_key("setMinPINLength"));
        if advertises_set_min_pin_length && !supports_client_pin {
            return Err(ProtocolError::UnexpectedType {
                field: "setMinPINLength",
                expected: "present only when the clientPin option is present",
            });
        }
        if supports_set_min_pin_length != self.max_rp_ids_for_set_min_pin_length.is_some() {
            return Err(ProtocolError::UnexpectedType {
                field: "maxRPIDsForSetMinPINLength",
                expected: "present if and only if the setMinPINLength option is true",
            });
        }
        let supports_credential_blob = self
            .extensions
            .as_ref()
            .is_some_and(|extensions| extensions.iter().any(|extension| extension == "credBlob"));
        if supports_credential_blob != self.max_credential_blob_length.is_some() {
            return Err(ProtocolError::UnexpectedType {
                field: "maxCredBlobLength",
                expected: "present if and only if credBlob is listed in extensions",
            });
        }
        let supports_large_blobs = self
            .options
            .as_ref()
            .and_then(|options| options.get("largeBlobs"))
            .copied()
            .unwrap_or(false);
        if supports_large_blobs != self.max_serialized_large_blob_array.is_some() {
            return Err(ProtocolError::UnexpectedType {
                field: "maxSerializedLargeBlobArray",
                expected: "present if and only if the largeBlobs option is true",
            });
        }
        if supports_large_blobs
            && self.extensions.as_ref().is_some_and(|extensions| {
                extensions.iter().any(|extension| extension == "largeBlob")
            })
        {
            return Err(ProtocolError::InvalidValue {
                field: "options.largeBlobs/extensions.largeBlob",
                expected: "at most one large-blob mechanism to be advertised",
            });
        }
        for (dependent, prerequisite, expected) in [
            (
                "noMcGaPermissionsWithClientPin",
                "clientPin",
                "present only when clientPin is present",
            ),
            (
                "uvBioEnroll",
                "bioEnroll",
                "present only when bioEnroll is present",
            ),
            (
                "uvAcfg",
                "authnrCfg",
                "present only when authnrCfg is present",
            ),
        ] {
            if option_present(&self.options, dependent)
                && !option_present(&self.options, prerequisite)
            {
                return Err(ProtocolError::InvalidValue {
                    field: dependent,
                    expected,
                });
            }
        }
        if option_is_true(&self.options, "alwaysUv")
            && self
                .options
                .as_ref()
                .and_then(|options| options.get("makeCredUvNotRqd"))
                != Some(&false)
        {
            return Err(ProtocolError::InvalidValue {
                field: "options.makeCredUvNotRqd",
                expected: "present and false when alwaysUv is true",
            });
        }
        // Enforcing a PIN policy and exposing it to RPs through the
        // pinComplexityPolicy extension are separate capabilities (CTAP 2.3 §7.5).
        Ok(())
    }
}

fn option_present(options: &Option<BTreeMap<String, bool>>, name: &str) -> bool {
    options
        .as_ref()
        .is_some_and(|options| options.contains_key(name))
}

fn option_is_true(options: &Option<BTreeMap<String, bool>>, name: &str) -> bool {
    options
        .as_ref()
        .and_then(|options| options.get(name))
        .copied()
        .unwrap_or(false)
}

fn require_unique<T>(values: &[T], field: &'static str) -> Result<(), ProtocolError>
where
    T: Eq + std::hash::Hash,
{
    let mut unique = HashSet::with_capacity(values.len());
    if values.iter().all(|value| unique.insert(value)) {
        Ok(())
    } else {
        Err(ProtocolError::DuplicateValue { field })
    }
}

fn require_non_empty_unique<T>(values: &[T], field: &'static str) -> Result<(), ProtocolError>
where
    T: Eq + std::hash::Hash,
{
    if values.is_empty() {
        return Err(ProtocolError::EmptyValue { field });
    }
    require_unique(values, field)
}

fn require_positive(value: Option<u64>, field: &'static str) -> Result<(), ProtocolError> {
    if value == Some(0) {
        Err(ProtocolError::MustBePositive { field })
    } else {
        Ok(())
    }
}

fn take_unsigned(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<u64>, ProtocolError> {
    fields
        .remove(&key)
        .map(|value| CtapValue::unsigned(value, field))
        .transpose()
}

fn take_positive(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<u64>, ProtocolError> {
    let result = take_unsigned(fields, key, field)?;
    if result == Some(0) {
        return Err(ProtocolError::MustBePositive { field });
    }
    Ok(result)
}

fn take_bool(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<bool>, ProtocolError> {
    fields
        .remove(&key)
        .map(|value| CtapValue::boolean(value, field))
        .transpose()
}

fn take_bytes(
    fields: &mut BTreeMap<u64, Value>,
    key: u64,
    field: &'static str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    fields
        .remove(&key)
        .map(|value| CtapValue::bytes(value, field))
        .transpose()
}

fn text_values(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::TextString).collect())
}

fn unsigned_values(values: &[u64]) -> Value {
    Value::Array(values.iter().copied().map(Value::Unsigned).collect())
}

fn transport_values(values: &[AuthenticatorTransport]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| Value::TextString(value.as_str().to_owned()))
            .collect(),
    )
}

fn bool_map(values: &BTreeMap<String, bool>) -> Value {
    Value::Map(
        values
            .iter()
            .map(|(key, value)| (Value::TextString(key.clone()), Value::bool_value(*value)))
            .collect(),
    )
}

fn unsigned_map(values: &BTreeMap<String, u64>) -> Value {
    Value::Map(
        values
            .iter()
            .map(|(key, value)| (Value::TextString(key.clone()), Value::Unsigned(*value)))
            .collect(),
    )
}

fn transport_array(
    value: Value,
    field: &'static str,
) -> Result<Vec<AuthenticatorTransport>, ProtocolError> {
    Ok(CtapValue::text_array(value, field, false)?
        .into_iter()
        .map(AuthenticatorTransport::from)
        .collect())
}

fn push_unsigned(fields: &mut Vec<(Value, Value)>, key: u64, value: Option<u64>) {
    if let Some(value) = value {
        fields.push((key.into(), value.into()));
    }
}

fn push_bool(fields: &mut Vec<(Value, Value)>, key: u64, value: Option<bool>) {
    if let Some(value) = value {
        fields.push((key.into(), value.into()));
    }
}

fn push_bytes(fields: &mut Vec<(Value, Value)>, key: u64, value: Option<&[u8]>) {
    if let Some(value) = value {
        fields.push((key.into(), Value::ByteString(value.to_vec())));
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    keyringd_testkit::scenario! {
        group "AuthenticatorGetInfoResponse::to_cbor_bytes" {
            case "Ctap 20 and 21 preview allow client pin without min pin length" {
                verify "accept missing minPINLength for CTAP 2.0 and the CTAP 2.1 preview" {
                    for version in [CtapVersion::Fido20, CtapVersion::Fido21Preview] {
                        let response =
                            AuthenticatorGetInfoResponse::new(vec![version.as_str().to_owned()], [0x11; 16])
                                .with_options(BTreeMap::from([
                                    ("clientPin".to_owned(), true),
                                    ("rk".to_owned(), true),
                                ]))
                                .with_pin_uv_auth_protocols(vec![1]);

                        let encoded = response.to_cbor_bytes().unwrap();
                        let decoded = AuthenticatorGetInfoResponse::from_cbor_bytes(&encoded).unwrap();

                        assert_eq!(decoded.highest_supported_version(), Some(version));
                        assert_eq!(decoded.min_pin_length(), None);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorGetInfoResponse::to_cbor_bytes" {
            case "Final ctap 21 and 23 require min pin length with client pin" {
                verify "reject missing minPINLength when client PIN is declared by CTAP 2.1 or 2.3" {
                    for version in [CtapVersion::Fido21, CtapVersion::Fido23] {
                        let response =
                            AuthenticatorGetInfoResponse::new(vec![version.as_str().to_owned()], [0x11; 16])
                                .with_options(BTreeMap::from([("clientPin".to_owned(), true)]));

                        assert!(matches!(
                            response.to_cbor_bytes(),
                            Err(ProtocolError::UnexpectedType {
                                field: "minPINLength",
                                ..
                            })
                        ));
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuthenticatorGetInfoResponse::from_cbor_bytes" {
            case "Rejects duplicate versions" {
                why "The specification does not explicitly forbid duplicate version strings, but the model rejects them to keep capability negotiation deterministic and unambiguous."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"
                given "Prepare a GetInfo response CBOR that has the same version" {
                    let bytes = hex_literal::hex!(
                        "a2
                         01 82
                            68 4649444f5f325f31
                            68 4649444f5f325f31
                         03 50
                            00000000000000000000000000000000"
                    );
                }
                when "Decode as a getInfo response" {
                    let result = AuthenticatorGetInfoResponse::from_cbor_bytes(&bytes);
                }
                then "Rejected as duplicate versions" {
                    assert!(matches!(
                        result,
                        Err(ProtocolError::DuplicateValue { field: "versions" })
                    ));
                }
            }
        }
    }
}
