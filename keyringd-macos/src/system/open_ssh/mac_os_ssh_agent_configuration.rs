use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::ssh_agent_socket::SshAgentSocket;

#[cfg(feature = "test-support")]
pub(crate) static FIXTURE_AGENT_SOCKET: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub struct MacOsSshAgentConfiguration {
    socket: SshAgentSocket,
}

impl MacOsSshAgentConfiguration {
    pub fn new(home: PathBuf, socket: PathBuf) -> Self {
        Self {
            socket: SshAgentSocket::new(home.join("Library/Application Support/keyringd"), socket),
        }
    }

    pub fn has_managed_agent_configuration(&self) -> io::Result<bool> {
        self.socket.is_own_link(&Self::native_socket()?)
    }

    pub fn configure(&self) -> io::Result<()> {
        self.validate_selection()?;
        self.socket.configure(&Self::native_socket()?)
    }

    pub(crate) fn validate_selection(&self) -> io::Result<()> {
        let source = Self::native_socket()?;
        let selected = Self::selected_socket()?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "The login session has not published SSH_AUTH_SOCK yet.",
            )
        })?;
        if !Self::same_path(Path::new(&selected), &source)? {
            return Err(io::Error::other(
                "SSH_AUTH_SOCK is managed by another agent. Keyringd left that agent unchanged.",
            ));
        }
        Ok(())
    }

    pub fn unconfigure(&self) -> io::Result<()> {
        self.socket.unconfigure(&Self::native_socket()?)
    }

    pub fn selected_socket() -> io::Result<Option<String>> {
        #[cfg(feature = "test-support")]
        if let Some(socket) = FIXTURE_AGENT_SOCKET.get() {
            return Ok(Some(socket.display().to_string()));
        }
        let output = Command::new("/bin/launchctl")
            .args(["getenv", "SSH_AUTH_SOCK"])
            .output()?;
        if !(output.status.success() || output.status.code() == Some(1) && output.stderr.is_empty())
        {
            return Err(io::Error::other(format!(
                "Could not read SSH_AUTH_SOCK: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let value = String::from_utf8(output.stdout).map_err(io::Error::other)?;
        Ok((!value.trim().is_empty()).then(|| value.trim().to_owned()))
    }

    fn native_socket() -> io::Result<PathBuf> {
        #[cfg(feature = "test-support")]
        if let Some(socket) = FIXTURE_AGENT_SOCKET.get() {
            return Ok(socket.clone());
        }
        // Use the actual system job, not a guessed launchd directory prefix.
        let job = format!("gui/{}/com.openssh.ssh-agent", unsafe { libc::getuid() });
        let output = Command::new("/bin/launchctl")
            .args(["print", &job])
            .output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "The macOS SSH agent is not registered. Enable com.openssh.ssh-agent before configuring Keyringd.",
            ));
        }
        let description = String::from_utf8(output.stdout).map_err(io::Error::other)?;
        let native = description
            .lines()
            .filter_map(|line| line.trim().strip_prefix("path = "))
            .map(PathBuf::from)
            .find(|path| path.file_name().is_some_and(|name| name == "Listeners"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "The macOS SSH agent socket is not ready.",
                )
            })?;
        Ok(native)
    }

    fn same_path(left: &Path, right: &Path) -> io::Result<bool> {
        // Compare directories without following the socket symlink itself.
        Ok(left.file_name() == right.file_name()
            && left.parent().map(Path::canonicalize).transpose()?
                == right.parent().map(Path::canonicalize).transpose()?)
    }
}
