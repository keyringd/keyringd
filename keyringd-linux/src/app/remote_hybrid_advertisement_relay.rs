use std::path::Path;

use anyhow::{Context, Result, bail};
use btleplug::api::{Central, CentralEvent, CentralState, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use keyringd_ssh_agent_protocol::extension_names::PXP_ADVERTISEMENT_RELAY_EXTENSION;

const FIDO_HYBRID_SERVICE: u128 = 0x0000fff9_0000_1000_8000_00805f9b34fb;
const GOOGLE_CABLE_SERVICE: u128 = 0x0000fde2_0000_1000_8000_00805f9b34fb;

pub(super) async fn run(agent_socket: &Path) -> Result<()> {
    let manager = Manager::new().await.context("open Bluetooth manager")?;
    let adapters = manager
        .adapters()
        .await
        .context("enumerate Bluetooth adapters")?;
    let mut selected = None;
    for adapter in adapters {
        if adapter
            .adapter_state()
            .await
            .context("read Bluetooth adapter state")?
            != CentralState::PoweredOff
        {
            selected = Some(adapter);
            break;
        }
    }
    let Some(adapter) = selected else {
        bail!("no powered Bluetooth adapter is available for Hybrid advertisement relay");
    };

    // Subscribe before starting discovery to retain short-lived service-data events.
    let mut events = adapter
        .events()
        .await
        .context("subscribe to Bluetooth events")?;
    adapter
        .start_scan(ScanFilter::default())
        .await
        .context("start Bluetooth scan")?;
    tracing::info!(socket = %agent_socket.display(), "forwarding Hybrid BLE advertisements to Agent");
    let result = async {
        let mut forwarding_failed = false;
        while let Some(event) = events.next().await {
            let CentralEvent::ServiceDataAdvertisement { service_data, .. } = event else {
                continue;
            };
            for (uuid, bytes) in service_data {
                let service = match uuid.as_u128() {
                    FIDO_HYBRID_SERVICE => 0xfff9_u16,
                    GOOGLE_CABLE_SERVICE => 0xfde2_u16,
                    _ => continue,
                };
                let Ok(bytes) = <[u8; 20]>::try_from(bytes.as_slice()) else {
                    tracing::debug!(length = bytes.len(), "ignored malformed Hybrid service data");
                    continue;
                };
                match send_advertisement(agent_socket, service, &bytes).await {
                    Ok(true) => {
                        if forwarding_failed {
                            tracing::debug!("Hybrid advertisement forwarding recovered");
                            forwarding_failed = false;
                        }
                    }
                    Ok(false) => bail!(
                        "Agent rejected Hybrid advertisements; start it with --accept-remote-hybrid-advertisements"
                    ),
                    Err(error) => {
                        if !forwarding_failed {
                            tracing::warn!(%error, "could not forward Hybrid advertisement");
                            forwarding_failed = true;
                        }
                    }
                }
            }
        }
        bail!("Bluetooth event stream ended while relaying Hybrid advertisements")
    }
    .await;
    adapter.stop_scan().await.context("stop Bluetooth scan")?;
    result
}

async fn send_advertisement(agent_socket: &Path, service: u16, bytes: &[u8; 20]) -> Result<bool> {
    let mut stream = UnixStream::connect(agent_socket)
        .await
        .with_context(|| format!("connect to Agent socket {}", agent_socket.display()))?;
    stream
        .write_all(&extension_frame(service, bytes))
        .await
        .context("send Agent extension")?;
    let length = stream
        .read_u32()
        .await
        .context("read Agent response length")?;
    if length != 1 {
        bail!("unexpected Agent response length: {length}");
    }
    match stream.read_u8().await.context("read Agent response")? {
        6 => Ok(true),
        5 => Ok(false),
        response => bail!("unexpected Agent response type: {response}"),
    }
}

fn extension_frame(service: u16, bytes: &[u8; 20]) -> Vec<u8> {
    let name = PXP_ADVERTISEMENT_RELAY_EXTENSION.as_bytes();
    let payload_length = 1 + 4 + name.len() + 2 + bytes.len();
    let mut frame = Vec::with_capacity(4 + payload_length);
    frame.extend_from_slice(&(payload_length as u32).to_be_bytes());
    frame.push(27);
    frame.extend_from_slice(&(name.len() as u32).to_be_bytes());
    frame.extend_from_slice(name);
    frame.extend_from_slice(&service.to_le_bytes());
    frame.extend_from_slice(bytes);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_ssh_agent_protocol::SshAgentRequest;

    keyringd_testkit::scenario! {
        group "extension_frame" {
            case "Relay frame is a standard agent extension request" {
                verify "decode the relay frame as an SSH agent extension request" {
                    let frame = extension_frame(0xfff9, &[0x42; 20]);
                    let length = u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize;
                    assert_eq!(length, frame.len() - 4);
                    let SshAgentRequest::Extension(request) = SshAgentRequest::from_bytes(&frame[4..]).unwrap()
                    else {
                        panic!("expected an Agent extension request");
                    };
                    assert_eq!(request.name(), PXP_ADVERTISEMENT_RELAY_EXTENSION);
                    assert_eq!(
                        request.name(),
                        "pxp-advertisement-relay-v01@keyringd.com"
                    );
                    assert_eq!(request.data(), [&[0xf9, 0xff][..], &[0x42; 20]].concat());
                }
            }
        }
    }
}
