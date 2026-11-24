/// The client parameters supplied by a case-owned SSH server.
#[derive(Clone)]
pub struct SshClientConfiguration {
    pub host: String,
    pub port: u16,
    pub use_agent: bool,
    pub user: String,
    #[cfg(windows)]
    pub config_file: std::path::PathBuf,
}
