use anyhow::{Context, Result, ensure};
use std::io::{BufReader, Read};
use std::process::{Child, Output};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Owns only the SSH client, its output readers, and one deadline from launch.
/// Server ownership and authentication evidence belong to the session.
pub struct SshClient {
    child: Child,
    stdout: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
    deadline: Instant,
}
impl SshClient {
    pub(crate) fn new(mut child: Child) -> Result<Self> {
        let stdout = child.stdout.take().context("missing SSH stdout");
        let stderr = child.stderr.take().context("missing SSH stderr");
        let (stdout, stderr) = match stdout.and_then(|out| stderr.map(|err| (out, err))) {
            Ok(streams) => streams,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        Ok(Self {
            child,
            stdout: Some(std::thread::spawn(move || {
                let mut bytes = Vec::new();
                BufReader::new(stdout)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
            })),
            stderr: Some(std::thread::spawn(move || {
                let mut bytes = Vec::new();
                BufReader::new(stderr)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
            })),
            deadline: Instant::now() + Duration::from_secs(30),
        })
    }
    pub fn is_finished(&mut self) -> Result<bool> {
        let finished = self.child.try_wait()?.is_some();
        ensure!(
            finished || Instant::now() < self.deadline,
            "SSH authentication timed out"
        );
        Ok(finished)
    }
    pub fn finish(mut self) -> Result<Output> {
        while !self.is_finished()? {
            std::thread::sleep(Duration::from_millis(20));
        }
        let status = self.child.wait()?;
        let stdout = self
            .stdout
            .take()
            .context("SSH stdout was already collected")?
            .join()
            .map_err(|_| anyhow::anyhow!("SSH stdout reader panicked"))??;
        let stderr = self
            .stderr
            .take()
            .context("SSH stderr was already collected")?
            .join()
            .map_err(|_| anyhow::anyhow!("SSH stderr reader panicked"))??;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}
impl Drop for SshClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.stdout.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr.take()
            && let Ok(Ok(stderr)) = reader.join()
            && !stderr.is_empty()
        {
            eprintln!(
                "unfinished SSH client diagnostics:\n{}",
                String::from_utf8_lossy(&stderr)
            );
        }
    }
}
