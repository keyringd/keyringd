use anyhow::{Result, ensure};
use std::process::{Child, Output};

pub struct WslClient {
    pub(super) child: Option<Child>,
}

impl WslClient {
    pub fn finish(mut self) -> Result<Output> {
        let output = self.child.take().unwrap().wait_with_output()?;
        ensure!(
            output.status.code() != Some(124),
            "WSL SSH client timed out: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output)
    }
}

impl Drop for WslClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            if let Ok(output) = child.wait_with_output()
                && std::thread::panicking()
            {
                eprintln!(
                    "WSL SSH client at failure:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
