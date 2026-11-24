use std::io;
use std::path::Path;
use std::time::Duration;

use keyringd_ssh_agent_protocol::SshAgentListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

use super::git_bash_socket_file::GitBashSocketFile;

pub(super) struct GitBashSocketListener {
    listener: TcpListener,
    secret: [u8; 16],
    handshakes: JoinSet<io::Result<TcpStream>>,
    _socket_file: GitBashSocketFile,
}

impl GitBashSocketListener {
    pub(super) async fn bind(path: &Path) -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let mut secret = [0; 16];
        getrandom::fill(&mut secret).map_err(io::Error::other)?;
        let socket_file = GitBashSocketFile::create(path, listener.local_addr()?.port(), &secret)?;
        Ok(Self {
            listener,
            secret,
            handshakes: JoinSet::new(),
            _socket_file: socket_file,
        })
    }
}

impl SshAgentListener for GitBashSocketListener {
    type Stream = TcpStream;

    async fn accept(&mut self) -> io::Result<TcpStream> {
        loop {
            tokio::select! {
                accepted = self.listener.accept(), if self.handshakes.len() < 16 => {
                    let (stream, _) = accepted?;
                    let secret = self.secret;
                    self.handshakes.spawn(async move {
                        tokio::time::timeout(Duration::from_secs(3), handshake(stream, secret))
                            .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Git for Windows socket handshake timed out"))?
                    });
                }
                completed = self.handshakes.join_next(), if !self.handshakes.is_empty() => {
                    match completed {
                        Some(Ok(Ok(stream))) => return Ok(stream),
                        Some(Ok(Err(error))) => tracing::debug!(%error, "Git for Windows socket handshake rejected"),
                        Some(Err(error)) => tracing::error!(%error, "Git for Windows socket handshake task failed"),
                        None => {}
                    }
                }
            }
        }
    }
}

async fn handshake(mut stream: TcpStream, secret: [u8; 16]) -> io::Result<TcpStream> {
    let mut received = [0; 16];
    stream.read_exact(&mut received).await?;
    // Compare every byte. Possession of the user-only rendezvous secret is the
    // access check; peer-reported POSIX credentials are not an authorization.
    if received
        .iter()
        .zip(secret)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid Git for Windows socket secret",
        ));
    }
    stream.write_all(&secret).await?;
    let mut credentials = [0; 12];
    stream.read_exact(&mut credentials).await?;
    // PID/UID/GID are three little-endian 32-bit values. MSYS clients use their
    // own UID/GID mapping; after authenticating the same Windows user through
    // the secret, preserve that mapping and report this server's Windows PID.
    credentials[..4].copy_from_slice(&std::process::id().to_le_bytes());
    stream.write_all(&credentials).await?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn client(listener: &GitBashSocketListener) -> TcpStream {
        let mut client = TcpStream::connect(listener.listener.local_addr().unwrap())
            .await
            .unwrap();
        client.write_all(&listener.secret).await.unwrap();
        client
    }

    keyringd_testkit::scenario! {
        group "GitBashSocketListener::bind" {
            case "Git bash login setting overrides inherited agent and reads identities" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "use the login shell's agent setting to list the fixture's identities" {
                    let directory = tempfile::Builder::new()
                        .prefix("keyringd-café-")
                        .tempdir()
                        .unwrap();
                    let data_dir = directory.path().join("keyringd");
                    std::fs::create_dir(&data_dir).unwrap();
                    let path = data_dir.join("agent.sock");
                    let mut listener = GitBashSocketListener::bind(&path).await.unwrap();
                    std::fs::write(
                        directory.path().join(".bash_profile"),
                        format!(
                            "{}\n",
                            crate::system::windows_ssh_client_configuration::GIT_FOR_WINDOWS_SSH_SETTING
                        ),
                    )
                    .unwrap();
                    let executable = std::path::PathBuf::from(std::env::var_os("ProgramFiles").unwrap())
                        .join("Git/usr/bin/ssh-add.exe");
                    assert!(
                        executable.is_file(),
                        "Git for Windows ssh-add is required for interoperability tests: {}",
                        executable.display()
                    );
                    let home = directory.path().to_owned();
                    let command = tokio::task::spawn_blocking(move || {
                        std::process::Command::new(executable.with_file_name("bash.exe"))
                            .args(["--login", "-c", "exec /usr/bin/ssh-add -l"])
                            .env("HOME", &home)
                            .env("LOCALAPPDATA", &home)
                            .env("SSH_AUTH_SOCK", r"\\.\pipe\invalid-inherited-agent")
                            .output()
                            .unwrap()
                    });
                    let mut stream = tokio::time::timeout(Duration::from_secs(10), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                    let mut request = [0; 5];
                    stream.read_exact(&mut request).await.unwrap();
                    assert_eq!(request, [0, 0, 0, 1, 11]);
                    stream
                        .write_all(&[0, 0, 0, 5, 12, 0, 0, 0, 0])
                        .await
                        .unwrap();
                    let output = command.await.unwrap();
                    assert_eq!(output.status.code(), Some(1));
                    assert!(
                        String::from_utf8_lossy(&output.stdout).contains("The agent has no identities."),
                        "{output:?}"
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "GitBashSocketListener::bind" {
            case "Rejected and stalled handshakes do not block a valid client" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "accept a valid client while failed handshakes are being rejected or timed out" {
                    let directory = tempfile::tempdir().unwrap();
                    let mut listener = GitBashSocketListener::bind(&directory.path().join("agent.sock"))
                        .await
                        .unwrap();
                    let mut rejected = TcpStream::connect(listener.listener.local_addr().unwrap())
                        .await
                        .unwrap();
                    rejected.write_all(&[0; 16]).await.unwrap();
                    let stalled = TcpStream::connect(listener.listener.local_addr().unwrap())
                        .await
                        .unwrap();
                    let mut valid = client(&listener).await;
                    let exchange = tokio::spawn(async move {
                        let mut echoed = [0; 16];
                        valid.read_exact(&mut echoed).await.unwrap();
                        valid.write_all(&[0; 12]).await.unwrap();
                        let mut credentials = [0; 12];
                        valid.read_exact(&mut credentials).await.unwrap();
                        valid.write_all(b"agent request").await.unwrap();
                        assert_eq!(&credentials[..4], &std::process::id().to_le_bytes());
                    });
                    let mut accepted = tokio::time::timeout(Duration::from_secs(1), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                    let mut request = [0; 13];
                    accepted.read_exact(&mut request).await.unwrap();
                    assert_eq!(&request, b"agent request");
                    exchange.await.unwrap();
                    assert_eq!(rejected.read(&mut [0]).await.unwrap(), 0);
                    // Drive accept until the stalled handshake is reaped.
                    tokio::time::timeout(Duration::from_secs(10), std::future::poll_fn(|cx| {
                        {
                            let accept = listener.accept();
                            tokio::pin!(accept);
                            assert!(accept.poll(cx).is_pending(), "no other client should be accepted");
                        }
                        if listener.handshakes.is_empty() {
                            std::task::Poll::Ready(())
                        } else {
                            std::task::Poll::Pending
                        }
                    })).await.expect("stalled handshake was not cleaned up");
                    drop(stalled);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "GitBashSocketListener::bind" {
            case "Endpoint is private non overwriting and removed on shutdown" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "create a private endpoint, retain existing files, and remove the owned file at shutdown" {
                    use std::os::windows::ffi::OsStrExt;
                    use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_SYSTEM, GetFileAttributesW};
                    let directory = tempfile::tempdir().unwrap();
                    let path = directory.path().join("agent.sock");
                    let listener = GitBashSocketListener::bind(&path).await.unwrap();
                    let original = std::fs::read(&path).unwrap();
                    assert!(original.starts_with(b"!<socket >"));
                    assert_private_file(&path);
                    let wide = path
                        .as_os_str()
                        .encode_wide()
                        .chain(Some(0))
                        .collect::<Vec<_>>();
                    assert_ne!(
                        unsafe { GetFileAttributesW(windows::core::PCWSTR(wide.as_ptr())) }
                            & FILE_ATTRIBUTE_SYSTEM.0,
                        0
                    );
                    assert!(GitBashSocketListener::bind(&path).await.is_err());
                    assert_eq!(std::fs::read(&path).unwrap(), original);
                    drop(listener);
                    assert!(!path.exists());
                    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
                    let restarted = GitBashSocketListener::bind(&path).await.unwrap();
                    assert_ne!(std::fs::read(&path).unwrap(), original);
                    drop(restarted);
                }
            }
        }
    }

    fn assert_private_file(path: &Path) {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
        use windows::Win32::Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
            SE_FILE_OBJECT,
        };
        use windows::Win32::Security::{DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR};
        use windows::core::PWSTR;
        let file = std::fs::File::open(path).unwrap();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = unsafe {
            GetSecurityInfo(
                HANDLE(file.as_raw_handle()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut descriptor),
            )
        };
        assert_eq!(status.0, 0);
        let mut encoded = PWSTR::null();
        unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut encoded,
                None,
            )
            .unwrap()
        };
        let sddl = unsafe { encoded.to_string().unwrap() };
        unsafe {
            let _ = LocalFree(Some(HLOCAL(encoded.0.cast())));
            let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        }
        let sid = crate::system::current_user_sid().unwrap();
        crate::app::handler::assert_user_only_access(&sddl, &sid);
    }
}
