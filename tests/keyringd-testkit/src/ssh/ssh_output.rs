use std::process::ExitStatus;

/// What `ssh` wrote.
#[derive(Debug)]
pub struct SshOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl From<std::process::Output> for SshOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}
