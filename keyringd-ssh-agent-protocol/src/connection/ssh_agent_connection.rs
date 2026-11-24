use std::collections::VecDeque;
use std::future::Future;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use super::SshAgentRequestHandler;
use crate::protocol::codec::SshWriter;
use crate::{SshAgentConfig, SshAgentResponse};

use super::{IncomingRequest, SshAgentRequestReader};

pub struct SshAgentConnection<S, H> {
    stream: S,
    handler: H,
    config: SshAgentConfig,
}

impl<S, H> SshAgentConnection<S, H> {
    pub fn new(stream: S, handler: H, config: SshAgentConfig) -> Self {
        Self {
            stream,
            handler,
            config,
        }
    }
}

impl<S, H> SshAgentConnection<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: SshAgentRequestHandler,
{
    pub async fn serve(self) -> std::io::Result<()> {
        let Self {
            mut stream,
            handler,
            config,
        } = self;
        let (reader, mut writer) = tokio::io::split(&mut stream);
        let mut reader = SshAgentRequestReader::new(reader, config.max_message_size());
        let mut pending = VecDeque::new();
        loop {
            let incoming = match pending.pop_front() {
                Some(request) => request,
                None => match reader.next().await? {
                    Some(request) => request,
                    None => return Ok(()),
                },
            };
            let response = async {
                match incoming {
                    Ok(request) => handler.handle(request).await,
                    Err(_) => SshAgentResponse::Failure,
                }
            };
            let Some(response) =
                response_or_disconnect(&mut reader, &mut pending, response, config).await?
            else {
                return Ok(());
            };
            let payload = response.to_bytes().map_err(invalid_data)?;
            writer.write_all(&SshWriter::frame(&payload)).await?;
        }
    }
}

async fn response_or_disconnect<R, T>(
    reader: &mut SshAgentRequestReader<R>,
    pending: &mut VecDeque<IncomingRequest>,
    response: T,
    config: SshAgentConfig,
) -> std::io::Result<Option<SshAgentResponse>>
where
    R: AsyncRead + Unpin,
    T: Future<Output = SshAgentResponse>,
{
    tokio::pin!(response);
    loop {
        tokio::select! {
            response = &mut response => return Ok(Some(response)),
            request = reader.next() => match request? {
                Some(request) if pending.len() < config.max_pending_requests() => {
                    pending.push_back(request);
                }
                Some(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "too many pending SSH Agent requests",
                    ));
                }
                None => return Ok(None),
            }
        }
    }
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {

    use crate::test_support::pending_handler;
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use super::*;
    use tokio::io::AsyncWriteExt;

    keyringd_testkit::scenario! {
        group "SshAgentConnection::serve" {
            case "Drop a request in progress on a Client disconnection" {
                why "To prevent unwanted side effects and resource retention without leaving incomplete processing of disconnected clients"
                async: tokio;
                timeout: Duration::from_secs(2);
                given "Prepare an SSH Agent connection with a handler that keeps the first Request Future in pending" {
                    let (mut client, server) = tokio::io::duplex(64);
                    let task = tokio::spawn(
                        SshAgentConnection::new(server, pending_handler(), SshAgentConfig::default()).serve(),
                    );
                }
                when "Client sends an Identity request and disconnects before the handler response" {
                    client.write_all(&SshWriter::frame(&[11])).await.unwrap();
                    drop(client);
                    let result = task.await.unwrap();
                }
                then "Server Drops Handler Future and Ends Connection Without Writing Response" {
                    assert!(result.is_ok());
                }
            }
            case "Limit the number of Request Queues waiting to be answered" {
                why "To prevent unlimited accumulation of Requests for handlers that have stopped responding"
                async: tokio;
                timeout: Duration::from_secs(2);
                given "Prepare an SSH Agent connection with a limited number of requests that can be Queue while waiting for the first response" {
                    let (mut client, server) = tokio::io::duplex(64);
                    let task = tokio::spawn(
                        SshAgentConnection::new(
                            server,
                            pending_handler(),
                            SshAgentConfig::default()
                                .with_max_message_size(NonZeroUsize::new(64).unwrap())
                                .with_max_pending_requests(1),
                        )
                        .serve(),
                    );
                    let request = SshWriter::frame(&[11]);
                }
                when "Three Identity requests arrive before the Handler completes the first response" {
                    client.write_all(&request).await.unwrap();
                    client.write_all(&request).await.unwrap();
                    client.write_all(&request).await.unwrap();
                    let result = task.await.unwrap();
                }
                then "Queue does not exceed the upper limit and returns a RequestQueueFull to the 3rd case" {
                    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
                }
            }
        }
    }
}
