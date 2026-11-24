use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::io::{AsyncWriteExt, copy};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

use crate::system::agent_pipe_name;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PIPE_BUSY: i32 = 231;
const AGENT_START_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(50);

pub struct StdioHandler;

impl StdioHandler {
    pub async fn run() -> std::io::Result<()> {
        let pipe = connect_or_start_agent().await?;
        let (mut pipe_read, mut pipe_write) = tokio::io::split(pipe);
        let mut stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let to_agent = async {
            copy(&mut stdin, &mut pipe_write).await?;
            pipe_write.shutdown().await
        };
        let from_agent = async {
            copy(&mut pipe_read, &mut stdout).await?;
            stdout.shutdown().await
        };
        tokio::pin!(to_agent);
        tokio::pin!(from_agent);
        tokio::select! { result = &mut to_agent => result, result = &mut from_agent => result }
    }
}

async fn connect_or_start_agent() -> std::io::Result<NamedPipeClient> {
    let pipe_name = agent_pipe_name()?;
    match ClientOptions::new().open(&pipe_name) {
        Ok(client) => return Ok(client),
        Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => start_agent()?,
        Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) => {}
        Err(error) => return Err(error),
    }
    let deadline = tokio::time::Instant::now() + AGENT_START_TIMEOUT;
    loop {
        match ClientOptions::new().open(&pipe_name) {
            Ok(client) => return Ok(client),
            Err(error) if is_waitable_connect_error(&error) => {}
            Err(error) => return Err(error),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timed out waiting for the Windows agent to start",
            ));
        }
        tokio::time::sleep(CONNECT_RETRY_INTERVAL).await;
    }
}

fn start_agent() -> std::io::Result<()> {
    let executable = std::env::current_exe()?;
    Command::new(&executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    tracing::info!(executable = %executable.display(), "started Windows agent on demand");
    Ok(())
}

fn is_waitable_connect_error(error: &std::io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(ERROR_FILE_NOT_FOUND | ERROR_PIPE_BUSY)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "is_waitable_connect_error" {
            case "Treats a missing or busy named pipe as retryable" {
                given "Prepare a Named Pipe Connection Error for Access Denied, uncreated, in use" {
                    let errors = [ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, 5].map(std::io::Error::from_raw_os_error);
                }
                when "Determine whether each ERROR is ready to wait" {
                    let waitable = errors.map(|error| is_waitable_connect_error(&error));
                }
                then "Can wait only if not created and in use" {
                    assert_eq!(waitable, [true, true, false]);
                }
            }
        }
    }
}
