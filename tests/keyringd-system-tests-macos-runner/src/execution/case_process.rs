use anyhow::{Context, Result};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

/// Owns a new process group: fixture/ssh/sshd descendants inherit that group.
/// Drop also handles errors in diagnostics or process polling.
pub(super) struct CaseProcess {
    pub(super) child: Child,
    pub(super) group: i32,
}
impl CaseProcess {
    pub(super) fn spawn(command: &mut Command) -> Result<Self> {
        let child = command
            .process_group(0)
            .spawn()
            .context("failed to start case process")?;
        let group = i32::try_from(child.id())?;
        Ok(Self { child, group })
    }
}
impl Drop for CaseProcess {
    fn drop(&mut self) {
        // This group was created for our child. Never signal the runner's group.
        unsafe {
            libc::killpg(self.group, libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
