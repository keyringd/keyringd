use tokio::io::{AsyncRead, AsyncReadExt};

use super::IncomingRequest;
use crate::SshAgentRequest;

/// Owns partially received frames across cancelled read futures.
pub(super) struct SshAgentRequestReader<R> {
    reader: R,
    buffer: Vec<u8>,
    max_message_size: usize,
}

impl<R: AsyncRead + Unpin> SshAgentRequestReader<R> {
    pub(super) fn new(reader: R, max_message_size: usize) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            max_message_size,
        }
    }

    pub(super) async fn next(&mut self) -> std::io::Result<Option<IncomingRequest>> {
        loop {
            let required = if self.buffer.len() < 4 {
                4
            } else {
                let length = u32::from_be_bytes(self.buffer[..4].try_into().unwrap()) as usize;
                if length > self.max_message_size {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SSH Agent message is too large",
                    ));
                }
                let required = length.checked_add(4).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SSH Agent message is too large",
                    )
                })?;
                if self.buffer.len() == required {
                    let request = SshAgentRequest::from_bytes(&self.buffer[4..]);
                    self.buffer.clear();
                    return Ok(Some(request));
                }
                required
            };
            let mut chunk = [0; 4096];
            let limit = chunk.len().min(required - self.buffer.len());
            // read is cancellation-safe; commit every completed read before awaiting again.
            let received = self.reader.read(&mut chunk[..limit]).await?;
            if received == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "incomplete SSH Agent frame",
                    ))
                };
            }
            self.buffer.extend_from_slice(&chunk[..received]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;
    use tokio::io::AsyncWriteExt;
    keyringd_testkit::scenario! {
        group "SshAgentRequestReader::next" {
            case "Cancelled reads preserve every partial header and payload boundary" {
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                verify "a following request remains intact after cancellation at every byte boundary" {
                    // An unknown request is still a complete frame with a preserved payload.
                    let frame = [0, 0, 0, 4, 255, 1, 2, 3];
                    for split in 1..frame.len() {
                        let (mut client, server) = tokio::io::duplex(64);
                        let mut reader = SshAgentRequestReader::new(server, 32);
                        client.write_all(&frame[..split]).await.unwrap();
                        {
                            let read = reader.next();
                            tokio::pin!(read);
                            std::future::poll_fn(|cx| {
                                assert!(matches!(read.as_mut().poll(cx), Poll::Pending));
                                Poll::Ready(())
                            }).await;
                        }
                        client.write_all(&frame[split..]).await.unwrap();
                        client.write_all(&[0, 0, 0, 1, 11]).await.unwrap();
                        assert!(matches!(reader.next().await.unwrap().unwrap().unwrap(), SshAgentRequest::Unknown { message_type: 255, .. }));
                        assert!(matches!(reader.next().await.unwrap().unwrap().unwrap(), SshAgentRequest::Identities));
                    }
                }
            }
        }
    }
}
