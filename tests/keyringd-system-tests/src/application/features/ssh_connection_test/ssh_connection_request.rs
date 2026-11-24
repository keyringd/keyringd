#[derive(Debug)]
pub struct SshConnectionRequest {
    pub host: String,
    pub public_identity: String,
}
