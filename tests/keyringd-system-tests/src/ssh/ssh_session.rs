use anyhow::Result;
use keyringd_testkit::ssh::SshCommandResult;

/// An owned SSH client and its server, independent of application UI operations.
pub trait SshSession {
    fn is_finished(&mut self) -> Result<bool>;
    fn finish(self) -> Result<SshCommandResult>;
}
