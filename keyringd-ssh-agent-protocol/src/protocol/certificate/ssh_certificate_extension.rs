#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshCertificateExtension {
    name: Vec<u8>,
    data: Vec<u8>,
}

impl SshCertificateExtension {
    pub fn new(name: impl AsRef<[u8]>, data: Vec<u8>) -> Self {
        Self {
            name: name.as_ref().to_vec(),
            data,
        }
    }

    pub fn empty(name: impl AsRef<[u8]>) -> Self {
        Self::new(name, Vec::new())
    }

    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
