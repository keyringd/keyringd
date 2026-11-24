use anyhow::{Result, ensure};
use keyringd_ssh_agent_protocol::extension_names::PXP_ADVERTISEMENT_RELAY_EXTENSION;
use std::path::PathBuf;

/// Sends QR-derived FFF9 service data to an Agent through its PXP extension.
/// The endpoint is a Unix socket on Unix and a Named Pipe on Windows.
pub struct PxpAdvertisementRelay {
    endpoint: PathBuf,
}

impl PxpAdvertisementRelay {
    pub fn new(endpoint: impl Into<PathBuf>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    #[cfg(unix)]
    pub async fn send(&self, service_data: &[u8; 20]) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::UnixStream::connect(&self.endpoint).await?;
        stream.write_all(&request(service_data)).await?;
        let mut response = [0; 5];
        stream.read_exact(&mut response).await?;
        check_response(response)
    }

    #[cfg(windows)]
    pub fn send(&self, service_data: &[u8; 20]) -> Result<()> {
        use std::io::{Read, Write};
        let mut stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.endpoint)?;
        stream.write_all(&request(service_data))?;
        let mut response = [0; 5];
        stream.read_exact(&mut response)?;
        check_response(response)
    }
}

fn request(service_data: &[u8; 20]) -> Vec<u8> {
    let name = PXP_ADVERTISEMENT_RELAY_EXTENSION.as_bytes();
    let length = 1 + 4 + name.len() + 2 + service_data.len();
    let mut frame = Vec::with_capacity(length + 4);
    frame.extend_from_slice(&(length as u32).to_be_bytes());
    frame.push(27); // SSH_AGENTC_EXTENSION
    frame.extend_from_slice(&(name.len() as u32).to_be_bytes());
    frame.extend_from_slice(name);
    frame.extend_from_slice(&0xfff9_u16.to_le_bytes());
    frame.extend_from_slice(service_data);
    frame
}

fn check_response(response: [u8; 5]) -> Result<()> {
    ensure!(
        response == [0, 0, 0, 1, 6],
        "Agent rejected PXP advertisement or returned an invalid response: {response:?}"
    );
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use keyringd_ssh_agent_protocol::SshAgentRequest;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Parse the actual wire request with the production codec, rather than
    /// reproducing its layout in the test.
    async fn exchange(response: &[u8]) -> Result<()> {
        let directory = tempfile::tempdir()?;
        let endpoint = directory.path().join("agent.sock");
        let listener = tokio::net::UnixListener::bind(&endpoint)?;
        let response = response.to_vec();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let length = socket.read_u32().await.unwrap() as usize;
            assert!(length < 1024);
            let mut payload = vec![0; length];
            socket.read_exact(&mut payload).await.unwrap();
            let SshAgentRequest::Extension(request) =
                SshAgentRequest::from_bytes(&payload).unwrap()
            else {
                panic!("expected Agent extension request");
            };
            assert_eq!(request.name(), PXP_ADVERTISEMENT_RELAY_EXTENSION);
            assert_eq!(&request.data()[..2], &[0xf9, 0xff]);
            assert_eq!(&request.data()[2..], &[0x42; 20]);
            socket.write_all(&response).await.unwrap();
        });
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            PxpAdvertisementRelay::new(endpoint).send(&[0x42; 20]),
        )
        .await?;
        peer.await?;
        result
    }

    keyringd_testkit::scenario! {
        group "PxpAdvertisementRelay::send" {
            case "Production codec accepts relay request" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "decode a relayed advertisement request with the production SSH agent codec" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                exchange(&[0, 0, 0, 1, 6]).await.unwrap();
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "PxpAdvertisementRelay::send" {
            case "Relay reports agent rejection and invalid responses" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return an error for both extension rejection and unexpected agent responses" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                assert!(exchange(&[0, 0, 0, 1, 5]).await.is_err());
                                assert!(exchange(&[0, 0, 0, 2, 6]).await.is_err());
                                assert!(exchange(&[0, 0, 0, 1]).await.is_err());
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
