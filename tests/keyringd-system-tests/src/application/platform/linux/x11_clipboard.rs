use std::time::{Duration, Instant};

use anyhow::{Result, ensure};

use crate::environment::linux::Environment;

pub struct X11Clipboard;

impl X11Clipboard {
    /// Replace any previous copy with an observed marker before invoking the UI.
    /// Waiting for this marker also confirms that xclip owns the X11 selection.
    pub(crate) async fn prepare_copy(environment: &Environment) -> Result<String> {
        let marker = format!("keyringd-copy-{}", uuid::Uuid::new_v4());
        let file = tempfile::NamedTempFile::new_in(environment.root())?;
        std::fs::write(file.path(), &marker)?;
        let status = environment
            .command("xclip")
            .args(["-selection", "clipboard", "-in"])
            .arg(file.path())
            // xclip forks the selection owner; it must not retain output pipes
            // that would keep Command::output waiting until the next copy.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await?;
        ensure!(
            status.success(),
            "failed to prepare X11 clipboard: {status}"
        );
        Self::wait_for_text(
            environment,
            |text| text == marker,
            "X11 clipboard did not receive the copy marker",
        )
        .await?;
        Ok(marker)
    }

    pub async fn read_text(environment: &Environment) -> Result<String> {
        Self::wait_for_text(environment, |_| true, "X11 clipboard is empty").await
    }

    pub async fn read_text_after(environment: &Environment, previous: &str) -> Result<String> {
        Self::wait_for_text(
            environment,
            |content| content != previous,
            "X11 clipboard was not updated",
        )
        .await
    }

    async fn wait_for_text(
        environment: &Environment,
        accept: impl Fn(&str) -> bool,
        timeout_message: &str,
    ) -> Result<String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let output = environment
                .run(
                    "xclip",
                    &["-selection", "clipboard", "-target", "UTF8_STRING", "-out"],
                )
                .await?;
            if output.status.success() {
                let content = output
                    .stdout
                    .trim_matches(|character: char| character.is_whitespace() || character == '\0');
                if !content.is_empty() && accept(content) {
                    return Ok(content.to_owned());
                }
            }
            ensure!(
                Instant::now() < deadline,
                "{timeout_message}: {}",
                output.stderr,
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
