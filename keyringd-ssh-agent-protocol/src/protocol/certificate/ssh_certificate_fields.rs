use crate::protocol::codec::{SshReader, SshWriter};
use crate::{
    SshAgentCodecError, SshCertificateCriticalOption, SshCertificateExtension, SshCertificateType,
    SshPublicKey,
};

/// Certificate fields following the algorithm-specific subject key fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshCertificateFields {
    pub serial: u64,
    pub certificate_type: SshCertificateType,
    pub key_id: Vec<u8>,
    pub valid_principals: Vec<Vec<u8>>,
    pub valid_after: u64,
    pub valid_before: u64,
    pub critical_options: Vec<SshCertificateCriticalOption>,
    pub extensions: Vec<SshCertificateExtension>,
    pub reserved: Vec<u8>,
    /// The CA's standalone public key, not a certificate.
    ///
    /// ```compile_fail
    /// use keyringd_ssh_agent_protocol::{SshCertificate, SshCertificateFields};
    /// fn nested_ca(fields: &mut SshCertificateFields, certificate: SshCertificate) {
    ///     fields.signature_key = certificate.to_key_blob();
    /// }
    /// ```
    pub signature_key: SshPublicKey,
}

impl SshCertificateFields {
    pub(super) fn validate(&self) -> Result<(), SshAgentCodecError> {
        if self.valid_before <= self.valid_after {
            return Err(SshAgentCodecError::InvalidValidityPeriod);
        }
        validate_name_order(
            self.critical_options
                .iter()
                .map(SshCertificateCriticalOption::name),
            "critical options",
        )?;
        validate_name_order(
            self.extensions.iter().map(SshCertificateExtension::name),
            "extensions",
        )?;
        Ok(())
    }

    pub(super) fn encode(&self, writer: &mut SshWriter) {
        writer.write_u64(self.serial);
        writer.write_u32(self.certificate_type.to_wire());
        writer.write_string(&self.key_id);
        let mut principals = SshWriter::new();
        for principal in &self.valid_principals {
            principals.write_string(principal);
        }
        writer.write_string(&principals.into_bytes());
        writer.write_u64(self.valid_after);
        writer.write_u64(self.valid_before);
        let mut options = SshWriter::new();
        for option in &self.critical_options {
            options.write_string(option.name());
            options.write_string(option.data());
        }
        writer.write_string(&options.into_bytes());
        let mut extensions = SshWriter::new();
        for extension in &self.extensions {
            extensions.write_string(extension.name());
            extensions.write_string(extension.data());
        }
        writer.write_string(&extensions.into_bytes());
        writer.write_string(&self.reserved);
        writer.write_string(&self.signature_key.to_bytes());
    }

    pub(super) fn read(reader: &mut SshReader<'_>) -> Result<Self, SshAgentCodecError> {
        let serial = reader
            .read_u64()
            .map_err(|_| SshAgentCodecError::Truncated { field: "serial" })?;
        let certificate_type = reader
            .read_u32()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "certificate type",
            })?;
        let certificate_type = SshCertificateType::from_wire(certificate_type).ok_or(
            SshAgentCodecError::Unsupported {
                field: "certificate type",
            },
        )?;
        let key_id = read_string(reader, "key ID")?.to_vec();
        let mut principals = SshReader::new(read_string(reader, "principals")?);
        let mut valid_principals = Vec::new();
        while !principals.remaining().is_empty() {
            valid_principals.push(read_string(&mut principals, "principal")?.to_vec());
        }
        let valid_after = reader
            .read_u64()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "validity timestamps",
            })?;
        let valid_before = reader
            .read_u64()
            .map_err(|_| SshAgentCodecError::Truncated {
                field: "validity timestamps",
            })?;
        let mut options = SshReader::new(read_string(reader, "critical options")?);
        let mut critical_options = Vec::new();
        while !options.remaining().is_empty() {
            let name = read_string(&mut options, "critical option name")?;
            let data = read_string(&mut options, "critical option data")?;
            critical_options.push(SshCertificateCriticalOption::new(name, data.to_vec()));
        }
        let mut extension_reader = SshReader::new(read_string(reader, "extensions")?);
        let mut extensions = Vec::new();
        while !extension_reader.remaining().is_empty() {
            let name = read_string(&mut extension_reader, "extension name")?;
            let data = read_string(&mut extension_reader, "extension data")?;
            extensions.push(SshCertificateExtension::new(name, data.to_vec()));
        }
        let reserved = read_string(reader, "reserved field")?.to_vec();
        let signature_key = SshPublicKey::from_bytes(read_string(reader, "signature key")?)?;
        Ok(Self {
            serial,
            certificate_type,
            key_id,
            valid_principals,
            valid_after,
            valid_before,
            critical_options,
            extensions,
            reserved,
            signature_key,
        })
    }
}

fn read_string<'a>(
    reader: &mut SshReader<'a>,
    field: &'static str,
) -> Result<&'a [u8], SshAgentCodecError> {
    reader
        .read_string()
        .map_err(|_| SshAgentCodecError::Truncated { field })
}

fn validate_name_order<'a>(
    names: impl Iterator<Item = &'a [u8]>,
    field: &'static str,
) -> Result<(), SshAgentCodecError> {
    let mut previous: Option<&[u8]> = None;
    for name in names {
        if name.is_empty() || previous.is_some_and(|previous| previous >= name) {
            return Err(SshAgentCodecError::InvalidValue {
                field,
                reason: "names must be non-empty, unique, and lexically ordered",
            });
        }
        previous = Some(name);
    }
    Ok(())
}
