use super::SshConnectionTestResult;
use std::io;
use std::path::Path;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::process::Command;

/// Owns the selected public identity file throughout an SSH connection test.
pub struct SshConnectionTest {
    identity: tempfile::TempPath,
}

impl SshConnectionTest {
    pub fn new(public_identity: &str) -> io::Result<Self> {
        use std::io::Write;
        let mut identity = tempfile::Builder::new()
            .prefix("keyringd-test-")
            .suffix(".pub")
            .tempfile()?;
        writeln!(identity, "{public_identity}")?;
        // Close the file handle so OpenSSH can also open it on Windows.
        Ok(Self {
            identity: identity.into_temp_path(),
        })
    }

    /// Bounds configuration/agent discovery as well as authentication subprocesses.
    pub async fn output(mut command: Command, timeout: Duration) -> io::Result<Output> {
        command
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command.spawn()?;
        // wait_with_output owns the child; dropping it kills the child on timeout
        // or when the enclosing connection test is cancelled.
        tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "OpenSSH operation timed out"))?
    }

    pub async fn execute(&self, command: Command, host: &str) -> SshConnectionTestResult {
        self.authenticate(command, host).await.unwrap_or_else(|error| {
            SshConnectionTestResult::ConnectionFailed(crate::util::report_user_error(
                &error,
                "Could not complete the SSH connection test. Check OpenSSH and the network, then try again.",
            ))
        })
    }

    async fn authenticate(
        &self,
        mut command: Command,
        host: &str,
    ) -> io::Result<SshConnectionTestResult> {
        // A ProxyJump subprocess shares stderr. Only the destination client's log
        // can confirm that the destination authenticated the user.
        let client_log = tempfile::NamedTempFile::new()?.into_temp_path();
        command.arg("-E").arg(&client_log);
        command.arg("-i").arg(&self.identity).args([
            "-vv",
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "KbdInteractiveAuthentication=no",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ConnectionAttempts=1",
            "-o",
            "SecurityKeyProvider=internal",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "ControlMaster=no",
            "-o",
            "ControlPath=none",
            "-o",
            "ControlPersist=no",
            "-o",
            "RemoteCommand=none",
            "--",
            host,
            "true",
        ]);
        let output = Self::output(command, Duration::from_secs(120)).await?;
        let log = std::fs::read(&client_log)?;
        let diagnostics = String::from_utf8_lossy(&log);
        Ok(classify_authentication(&diagnostics, &self.identity).unwrap_or_else(|| {
            tracing::warn!(
                error = %diagnostics,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "SSH connection test failed"
            );
            SshConnectionTestResult::ConnectionFailed(
                "Could not connect to the SSH server. Check the host name, network connection, and server settings, then try again.".into(),
            )
        }))
    }
}

fn classify_authentication(diagnostics: &str, identity: &Path) -> Option<SshConnectionTestResult> {
    let mut selected = false;
    let mut offered_selected = false;
    let mut selected_key_completed_factor = false;
    for line in diagnostics.lines() {
        if let Some(key) = line.strip_prefix("debug1: Offering public key: ") {
            offered_selected |= names_identity(key, identity);
            selected = false;
        } else if line.starts_with("debug1: Trying private key: ")
            || line.contains("sign_and_send_pubkey: signing failed")
        {
            selected = false;
        } else if let Some(key) = line.strip_prefix("debug1: Server accepts key: ") {
            // OpenSSH logs the identity filename before signing. Check the exact
            // filename, and reset it on subsequent attempts, rather than accepting
            // a successful login using another IdentityFile in the user's config.
            // See openssh-portable/sshconnect2.c: input_userauth_pk_ok / userauth_pubkey.
            selected = names_identity(key, identity);
        } else if matches!(
            line,
            r#"Authenticated using "publickey" with partial success."#
                | "Authenticated with partial success."
        ) {
            // Some OpenSSH clients omit the method here. This test requests only publickey.
            selected_key_completed_factor |= selected;
        } else if line.starts_with("Authenticated to ")
            || line.starts_with("debug1: Authentication succeeded (")
        {
            let used_public_key =
                line.ends_with(r#" using "publickey"."#) || line.ends_with("(publickey).");
            let result = if selected_key_completed_factor || (selected && used_public_key) {
                SshConnectionTestResult::Connected
            } else {
                SshConnectionTestResult::SelectedIdentityUnconfirmed
            };
            return Some(result);
        }
    }
    if selected_key_completed_factor {
        Some(SshConnectionTestResult::AdditionalAuthenticationRequired)
    } else if offered_selected
        && diagnostics.contains("Permission denied (")
        && diagnostics.contains("publickey")
        && !diagnostics.contains("sign_and_send_pubkey: signing failed")
    {
        Some(SshConnectionTestResult::IdentityRejected)
    } else {
        None
    }
}

fn names_identity(key: &str, identity: &Path) -> bool {
    // Filenames may contain spaces. Compare complete path prefixes using native
    // Path semantics, also accepting OpenSSH's escaped backslashes in diagnostics.
    key.match_indices(' ').any(|(end, _)| {
        let name = &key[..end];
        Path::new(name) == identity || Path::new(&name.replace("\\\\", "\\")) == identity
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "classify_authentication" {
            case "Connection success requires authentication with the selected identity" {
                verify "other configured identities connection reuse and signing failures cannot pass" {
                    let selected = Path::new("/tmp/selected key.pub");
                    let accepted = "debug1: Server accepts key: /tmp/selected key.pub ECDSA SHA256:abc explicit agent\n";
                    let authenticated = "Authenticated to host ([127.0.0.1]:22) using \"publickey\".\n";
                    let partial = "Authenticated using \"publickey\" with partial success.\nuser@host: Permission denied (keyboard-interactive).\n";
                    let unconfirmed = Some(SshConnectionTestResult::SelectedIdentityUnconfirmed);
                    assert_eq!(classify_authentication(&format!("{accepted}{partial}"), selected), Some(SshConnectionTestResult::AdditionalAuthenticationRequired));
                    for other in [
                        "debug1: Server accepts key: /tmp/other.pub ECDSA SHA256:def explicit agent\n",
                        "debug1: Offering public key: /tmp/other.pub ECDSA SHA256:def explicit agent\n",
                        "debug1: Trying private key: /tmp/other\n",
                        "sign_and_send_pubkey: signing failed\n",
                        "debug1: Server accepts key: /tmp/selected key.pub-other ECDSA SHA256:def agent\n",
                    ] {
                        assert_eq!(classify_authentication(&format!("{accepted}{other}{authenticated}"), selected), unconfirmed);
                    }
                    let native = Path::new("directory").join("selected key.pub");
                    let native_name = native.to_string_lossy();
                    for logged in [
                        native_name.to_string(),
                        native_name.replace('\\', "/"),
                        native_name.replace('\\', "\\\\"),
                    ] {
                        let accepted = format!("debug1: Server accepts key: {logged} ECDSA SHA256:abc explicit agent\n");
                        assert_eq!(classify_authentication(&format!("{accepted}{authenticated}"), &native), Some(SshConnectionTestResult::Connected));
                        assert_eq!(classify_authentication(&format!("debug1: Server accepts key: {logged}-other ECDSA explicit agent\n{authenticated}"), &native), unconfirmed);
                    }
                    assert_eq!(classify_authentication("debug1: Offering public key: /tmp/other.pub ECDSA explicit agent\nPermission denied (publickey).", selected), None);
                    assert_eq!(classify_authentication(authenticated, selected), unconfirmed);
                    assert_eq!(classify_authentication("debug1: Offering public key: /tmp/selected key.pub ECDSA-CERT explicit agent\nuser@host: Permission denied (publickey,password).", selected), Some(SshConnectionTestResult::IdentityRejected));
                    assert_eq!(classify_authentication("debug1: Offering public key: /tmp/selected key.pub ECDSA-CERT explicit agent\nsign_and_send_pubkey: signing failed\nuser@host: Permission denied (publickey).", selected), None);
                }
            }
        }
    }
}
