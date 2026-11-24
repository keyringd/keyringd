use super::current_user::CurrentUser;
use super::file_security_descriptor::FileSecurityDescriptor;
use anyhow::{Result, ensure};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub(super) fn prepare(directory: &Path, binaries: &Path, agent: &str) -> Result<()> {
    let user = CurrentUser::new()?;
    let authorized = directory.join("authorized_keys");
    FileSecurityDescriptor::user_and_system(&user.sid_string()?)?.apply(&authorized)?;
    let host_key = directory.join("host_key");
    let mut keygen = Command::new(binaries.join("ssh-keygen.exe"));
    keygen
        .args(["-q", "-t", "ecdsa", "-b", "256", "-N", "", "-f"])
        .arg(&host_key);
    let output =
        crate::environment::windows::command_output::run_command(keygen, Duration::from_secs(10))?;
    ensure!(
        output.status.success(),
        "SSH host key generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    let user = user.ssh_account_name()?;
    // Both native Windows and Git's MSYS SSH use this fixture's actual host
    // key. Avoid Windows-only NUL paths, which MSYS treats as ordinary files.
    let known_hosts = directory.join("known_hosts");
    std::fs::write(
        &known_hosts,
        format!(
            "[127.0.0.1]:{port} {}",
            std::fs::read_to_string(directory.join("host_key.pub"))?
        ),
    )?;
    let known_hosts = known_hosts.to_string_lossy().replace('\\', "/");
    let host_key = host_key.to_string_lossy().replace('\\', "/");
    let authorized = authorized.to_string_lossy().replace('\\', "/");
    std::fs::write(
        directory.join("sshd_config"),
        format!(
            "Port {port}\nListenAddress 127.0.0.1\nHostKey \"{host_key}\"\n\
         AuthorizedKeysFile \"{authorized}\"\nPubkeyAuthentication yes\nPasswordAuthentication no\n\
         KbdInteractiveAuthentication no\nStrictModes yes\nAllowUsers {user}\nLogLevel VERBOSE\n"
        ),
    )?;
    let agent = agent.replace('\\', "/");
    std::fs::write(
        directory.join("ssh_config"),
        format!(
            "Host ui-test-host\n    HostName 127.0.0.1\n    Port {port}\n    User {user}\n\
             IdentityAgent {agent}\n    IdentityFile none\n    IdentitiesOnly yes\n\
             BatchMode yes\n    StrictHostKeyChecking yes\n    UserKnownHostsFile \"{known_hosts}\"\n\
             GlobalKnownHostsFile none\n    ConnectTimeout 3\n    ConnectionAttempts 1\n\
             ControlMaster no\n    ControlPath none\n"
        ),
    )?;
    Ok(())
}
