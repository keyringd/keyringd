//! Reads the product Agent through its own socket.
//!
//! The socket is the one this fixture asked the product to create inside the
//! case's temporary directory, so `SSH_AUTH_SOCK` is set for the one command
//! and the developer's own agent is neither read nor replaced.

use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};

use crate::environment::macos::TestApp;

const TIMEOUT: Duration = Duration::from_secs(10);
const SETTLE: Duration = Duration::from_millis(100);

/// What `ssh-add -L` answered.
pub struct AgentListing {
    pub identities: String,
    pub message: String,
    /// `ssh-add` exits non-zero when the Agent publishes nothing, which is an
    /// answer a case may be asking for rather than a failure to ask.
    pub succeeded: bool,
}

impl AgentListing {
    /// How many published identities name `profile`, which is how a case reads
    /// what one profile contributes.
    pub fn count(&self, profile: &str) -> usize {
        self.identities
            .lines()
            .filter(|line| {
                line.splitn(3, ' ')
                    .nth(2)
                    .is_some_and(|comment| comment.starts_with(&format!("keyringd {profile} ")))
            })
            .count()
    }

    pub fn report(&self) -> String {
        format!("stdout:\n{}\nstderr:\n{}", self.identities, self.message)
    }
}

/// Lists what the product Agent publishes right now.
pub fn list_identities(app: &TestApp) -> Result<AgentListing> {
    let socket = app.agent_socket()?;
    let output = Command::new("ssh-add")
        .arg("-L")
        .env("SSH_AUTH_SOCK", &socket)
        .env("LC_ALL", "C")
        .output()
        .context("failed to run ssh-add")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    ensure!(
        output.status.success()
            || (output.status.code() == Some(1)
                && (stdout.trim() == "The agent has no identities."
                    || stderr.trim() == "The agent has no identities.")),
        "could not read Agent identities: {stdout}\n{stderr}"
    );
    Ok(AgentListing {
        identities: if output.status.success() {
            stdout.into_owned()
        } else {
            String::new()
        },
        message: stderr.into_owned(),
        succeeded: output.status.success(),
    })
}

/// Waits until the Agent publishes exactly `expected` identities naming
/// `profile`. The Agent follows a UI change on its own thread, so the count a
/// case is after is reached a moment after the window shows it.
pub fn wait_identities(app: &TestApp, profile: &str, expected: usize) -> Result<AgentListing> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let listing = list_identities(app)?;
        if listing.count(profile) == expected {
            return Ok(listing);
        }
        ensure!(
            Instant::now() < deadline,
            "timed out waiting for {expected} identities of {profile:?}; the Agent answered {}",
            listing.report()
        );
        std::thread::sleep(SETTLE);
    }
}
