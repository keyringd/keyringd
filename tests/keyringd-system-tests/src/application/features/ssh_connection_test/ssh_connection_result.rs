/// Text and actions actually exposed by the connection result window.
/// Expected titles, diagnostics and registration instructions belong to the case.
#[derive(Debug)]
pub struct SshConnectionResult {
    pub title: String,
    pub host: String,
    pub diagnostic: String,
    pub rejection: Option<SshRejectionGuidance>,
}

#[derive(Debug)]
pub struct SshRejectionGuidance {
    pub entry: String,
    pub instructions: String,
}
