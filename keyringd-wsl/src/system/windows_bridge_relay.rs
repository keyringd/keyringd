use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncWriteExt, copy};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};

const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub(crate) struct WindowsBridgeRelay {
    executable: PathBuf,
}

impl WindowsBridgeRelay {
    pub(crate) fn new(configured_executable: Option<PathBuf>) -> std::io::Result<Self> {
        let executable = match configured_executable {
            Some(path) if path.is_file() => path,
            Some(path) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "configured Windows executable not found at {}",
                        path.display()
                    ),
                ));
            }
            None => Self::find_adjacent_executable(&std::env::current_exe()?)?,
        };
        Ok(Self { executable })
    }

    pub(crate) async fn relay(&self, stream: UnixStream) -> std::io::Result<()> {
        let mut command = Command::new(&self.executable);
        command.arg("bridge");
        Self::relay_child(stream, command, SHUTDOWN_GRACE).await
    }

    fn find_adjacent_executable(helper: &Path) -> std::io::Result<PathBuf> {
        let directory = helper.parent().unwrap_or_else(|| Path::new("."));
        let adjacent = directory.join("keyringd.exe");
        if adjacent.is_file() {
            return Ok(adjacent);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Windows executable not found at {}", adjacent.display()),
        ))
    }

    async fn relay_child(
        stream: UnixStream,
        mut command: Command,
        shutdown_grace: Duration,
    ) -> std::io::Result<()> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;
        let mut child_stdin = child.stdin.take().expect("stdin was set to piped");
        let mut child_stdout = child.stdout.take().expect("stdout was set to piped");
        let (mut socket_read, mut socket_write) = stream.into_split();

        let to_windows = async {
            copy(&mut socket_read, &mut child_stdin).await?;
            child_stdin.shutdown().await
        };
        let from_windows = async {
            copy(&mut child_stdout, &mut socket_write).await?;
            socket_write.shutdown().await
        };

        let transfer_result = {
            tokio::pin!(to_windows);
            tokio::pin!(from_windows);
            tokio::select! {
                result = &mut to_windows => match result {
                    Ok(()) => match tokio::time::timeout(shutdown_grace, &mut from_windows).await {
                        Ok(result) => result,
                        Err(_) => Ok(()),
                    },
                    Err(error) => Err(error),
                },
                result = &mut from_windows => result,
            }
        };

        drop(child_stdin);
        let child_result = Self::reap_child(&mut child, shutdown_grace).await;
        // Keep the pipe open until the child exits so cleanup cannot cause SIGPIPE.
        drop(child_stdout);
        if transfer_result.is_err()
            && let Err(error) = &child_result
        {
            tracing::warn!(
                ?error,
                "Windows bridge cleanup failed after a transfer failure"
            );
        }
        transfer_result.and(child_result)
    }

    async fn reap_child(child: &mut Child, shutdown_grace: Duration) -> std::io::Result<()> {
        match tokio::time::timeout(shutdown_grace, child.wait()).await {
            Ok(result) => {
                let status = result?;
                if status.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!(
                        "Windows bridge process exited unsuccessfully: {status}"
                    )))
                }
            }
            Err(_) => {
                child.kill().await?;
                child.wait().await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(feature = "test-support")]
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    keyringd_testkit::scenario! {
        group "WindowsBridgeRelay::relay_child" {
            case "Relay reports an unsuccessful bridge exit" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "surface the nonzero exit status as a relay error" {
                    let (_client, server) = UnixStream::pair().unwrap();
                    let mut command = Command::new("sh");
                    command.args(["-c", "exit 7"]);
                    let error = WindowsBridgeRelay::relay_child(server, command, Duration::from_secs(1))
                        .await
                        .unwrap_err();
                    assert!(error.to_string().contains("exited unsuccessfully"));
                    assert!(error.to_string().contains('7'));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "WindowsBridgeRelay::relay_child" {
            case "Relay accepts a successful bridge exit" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "complete the relay successfully when the bridge exits with status zero" {
                    let (_client, server) = UnixStream::pair().unwrap();
                    let mut command = Command::new("sh");
                    command.args(["-c", "exit 0"]);
                    WindowsBridgeRelay::relay_child(server, command, Duration::from_secs(1))
                        .await
                        .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "WindowsBridgeRelay::find_adjacent_executable" {
            case "Finds the Windows executable beside the portable helper" {
                given "There is a Windows run file in the same directory as Portable Helper" {
                    let directory = tempfile::tempdir().unwrap();
                    let helper = directory.path().join("keyringd");
                    let windows_executable = directory.path().join("keyringd.exe");
                    fs::write(&helper, []).unwrap();
                    fs::write(&windows_executable, []).unwrap();
                }
                when "Find Windows Run File from Helper's Path" {
                    let found = WindowsBridgeRelay::find_adjacent_executable(&helper).unwrap();
                }
                then "Returns an EXE file of the same directory" {
                    assert_eq!(found, windows_executable);
                }
            }
        }
    }

    #[cfg(feature = "test-support")]
    keyringd_testkit::scenario! {
        group "WindowsBridgeRelay::relay_child" {
            case "Relay collects child process after client disconnection" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "There is a UNIX Stream Relay connected to Cat Process" {
                    let (mut client, server) = UnixStream::pair().unwrap();
                    let command = Command::new("cat");
                    let relay = tokio::spawn(WindowsBridgeRelay::relay_child(
                        server,
                        command,
                        Duration::from_millis(200),
                    ));
                }
                when "Disconnect the Client after reciprocating Data" {
                    client.write_all(b"ping").await.unwrap();
                    let mut response = [0; 4];
                    client.read_exact(&mut response).await.unwrap();
                }
                then "Returns a ping sent from Windows Bridge" {
                    assert_eq!(&response, b"ping");
                }
                when "Close the Windows Bridge Client connection" {
                    client.shutdown().await.unwrap();
                    drop(client);
                }
                then "Relay ends up collecting child process within the deadline" {
                    tokio::time::timeout(Duration::from_secs(1), relay)
                        .await
                        .expect("relay did not finish")
                        .unwrap()
                        .unwrap();
                }
            }
        }
    }

    #[cfg(feature = "test-support")]
    keyringd_testkit::scenario! {
        group "WindowsBridgeRelay::relay_child" {
            case "Relay ends a child process that ignores standard input EOF" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "There is a relay of the child process that does not terminate with the standard input EOF" {
                    let (mut client, server) = UnixStream::pair().unwrap();
                    let mut command = Command::new("sleep");
                    command.arg("infinity");
                    let relay = tokio::spawn(WindowsBridgeRelay::relay_child(
                        server,
                        command,
                        Duration::from_millis(50),
                    ));
                }
                when "Disconnect the Client" {
                    client.shutdown().await.unwrap();
                    drop(client);
                }
                then "End and collect child process after the grace period" {
                    tokio::time::timeout(Duration::from_secs(1), relay)
                        .await
                        .expect("relay did not reap stubborn child")
                        .unwrap()
                        .unwrap();
                }
            }
        }
    }
}
