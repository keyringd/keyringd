use anyhow::Result;
use std::path::PathBuf;

/// Observations and setup for real service-manager socket activation.
/// Scenarios own the connection attempts, waiting, and assertions.
pub trait SocketActivationDriver: Sized {
    fn prepare() -> Result<Self>;
    fn socket_path(&self) -> Result<PathBuf>;
    fn process_id(&self) -> Result<Option<u32>>;
}
