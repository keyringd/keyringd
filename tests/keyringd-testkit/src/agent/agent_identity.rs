use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};

/// An actual Agent observation, independent of socket, pipe, or UI APIs.
#[derive(Debug, Eq, PartialEq)]
pub struct AgentIdentity {
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub comment: String,
}

impl AgentIdentity {
    pub fn to_openssh(&self) -> String {
        format!(
            "{} {} {}",
            self.algorithm,
            STANDARD.encode(&self.public_key),
            self.comment
        )
    }
    pub fn from_openssh(line: &str) -> Result<Self> {
        let mut fields = line.splitn(3, ' ');
        let algorithm = fields.next().context("missing identity algorithm")?;
        let public_key = STANDARD.decode(fields.next().context("missing identity key")?)?;
        ensure!(!public_key.is_empty(), "empty identity key");
        Ok(Self {
            algorithm: algorithm.to_owned(),
            public_key,
            comment: fields
                .next()
                .context("missing identity comment")?
                .to_owned(),
        })
    }
}
