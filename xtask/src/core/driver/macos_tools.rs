use std::io::{Read, Seek};
use std::process::{Command, Output, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};

/// Runs native tools without logging command arguments (which can contain credentials).
/// File-backed output avoids pipe deadlocks and lets cancellation unwind signing guards.
pub(crate) struct MacosTools {
    cancelled: Arc<AtomicBool>,
    secrets: Vec<String>,
}

impl MacosTools {
    pub(crate) fn new() -> Result<Self> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))?;
        Ok(Self {
            cancelled,
            secrets: Vec::new(),
        })
    }

    pub(crate) fn mask(&mut self, secret: &str) {
        if !secret.is_empty() {
            self.secrets.push(secret.to_owned());
        }
    }

    pub(crate) fn output(&self, command: &mut Command) -> Result<Output> {
        ensure!(
            !self.cancelled.load(Ordering::SeqCst),
            "macOS packaging interrupted"
        );
        let mut stdout = tempfile::tempfile()?;
        let mut stderr = tempfile::tempfile()?;
        let mut child = command
            .stdin(Stdio::null())
            .stdout(stdout.try_clone()?)
            .stderr(stderr.try_clone()?)
            .spawn()
            .with_context(|| format!("failed to run {:?}", command.get_program()))?;
        let status = loop {
            if self.cancelled.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                bail!("macOS packaging interrupted");
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.into());
                }
            }
        };
        stdout.rewind()?;
        stderr.rewind()?;
        let mut output = Output {
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        stdout.read_to_end(&mut output.stdout)?;
        stderr.read_to_end(&mut output.stderr)?;
        Ok(output)
    }

    pub(crate) fn checked(&self, command: &mut Command) -> Result<Output> {
        let output = self.output(command)?;
        self.check_output(command, output)
    }

    fn check_output(&self, command: &Command, output: Output) -> Result<Output> {
        ensure!(
            output.status.success(),
            "{:?} exited with {}\nstdout:\n{}\nstderr:\n{}",
            command.get_program(),
            output.status,
            self.redact(&String::from_utf8_lossy(&output.stdout)),
            self.redact(&String::from_utf8_lossy(&output.stderr))
        );
        Ok(output)
    }

    pub(crate) fn text(&self, command: &mut Command) -> Result<String> {
        Ok(String::from_utf8(self.checked(command)?.stdout)?
            .trim()
            .to_owned())
    }

    pub(crate) fn redact(&self, text: &str) -> String {
        self.secrets.iter().fold(text.to_owned(), |text, secret| {
            text.replace(secret, "[REDACTED]")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "MacosTools::check_output" {
            case "Failed tools report both streams without disclosing secrets or arguments" {
                verify "include both output streams in diagnostics and omit sensitive command details" {
                    #[cfg(unix)]
                    use std::os::unix::process::ExitStatusExt;
                    #[cfg(windows)]
                    use std::os::windows::process::ExitStatusExt;

                    #[cfg(unix)]
                    let status = std::process::ExitStatus::from_raw(68 << 8);
                    #[cfg(windows)]
                    let status = std::process::ExitStatus::from_raw(68);

                    let tools = MacosTools {
                        cancelled: Arc::new(AtomicBool::new(false)),
                        secrets: vec!["synthetic-password".into()],
                    };
                    let mut command = Command::new("xcrun");
                    command.args(["stapler", "staple", "private-argument"]);
                    let error = tools
                        .check_output(
                            &command,
                            Output {
                                status,
                                stdout: b"ticket lookup failed: synthetic-password".to_vec(),
                                stderr: b"diagnostic: synthetic-password".to_vec(),
                            },
                        )
                        .unwrap_err()
                        .to_string();
                    assert!(error.contains("xcrun"));
                    assert!(error.contains("68"));
                    assert!(error.contains("stdout:\nticket lookup failed: [REDACTED]"));
                    assert!(error.contains("stderr:\ndiagnostic: [REDACTED]"));
                    assert!(!error.contains("synthetic-password"));
                    assert!(!error.contains("private-argument"));
                }
            }
        }
    }
}
