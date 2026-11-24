use anyhow::{Context, Result, bail, ensure};
use bt_hci::cmd::{
    controller_baseband::Reset,
    le::{LeSetAdvData, LeSetAdvEnable, LeSetAdvParams},
};
use bt_hci::param::{
    AddrKind, AdvChannelMap, AdvFilterPolicy, AdvKind, BdAddr, Duration as HciDuration,
};
use bt_hci::{
    FromHciBytes, WriteHci,
    cmd::SyncCmd,
    event::{CommandCompleteWithStatus, Event},
};
use std::{net::SocketAddr, time::Duration};
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use keyringd_test_usbip::authenticator_server::client::HybridAdvertisement;

/// Owns the Phone's socket controller, which is invisible to kernel/BlueZ.
/// Dropping the connection removes the controller and stops its advertising.
pub struct VirtualBleAdvertiser {
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    address: SocketAddr,
    connection: Mutex<Option<HciConnection>>,
}

impl VirtualBleAdvertiser {
    pub(crate) fn from_environment(
        runtime: std::sync::Arc<tokio::runtime::Runtime>,
    ) -> Result<Self> {
        let address: SocketAddr = std::env::var("KEYRINGD_E2E_BLE_ADVERTISER_ADDRESS")
            .context("virtual BLE advertisement requires btvirt in the Testbed")?
            .parse()
            .context("invalid btvirt Phone HCI address")?;
        ensure!(
            address.ip().is_loopback(),
            "btvirt Phone must use localhost"
        );
        Ok(Self {
            runtime,
            address,
            connection: Mutex::new(None),
        })
    }

    /// Publishes QR-derived FFF9 service data for BlueZ/btleplug to scan.
    pub fn advertise(&self, advertisement: &HybridAdvertisement) -> Result<()> {
        self.runtime.block_on(self.publish(advertisement))
    }

    async fn publish(&self, advertisement: &HybridAdvertisement) -> Result<()> {
        let service_data = advertisement.service_data()?;
        let mut connection = self.connection.lock().await;
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            if connection.is_none() {
                let mut phone = HciConnection::connect(self.address).await?;
                phone.command(Reset::new()).await?;
                phone
                    .command(LeSetAdvParams::new(
                        HciDuration::from_millis(100),
                        HciDuration::from_millis(100),
                        AdvKind::AdvNonconnInd,
                        AddrKind::PUBLIC,
                        AddrKind::PUBLIC,
                        BdAddr::new([0; 6]),
                        AdvChannelMap::ALL,
                        AdvFilterPolicy::Unfiltered,
                    ))
                    .await?;
                *connection = Some(phone);
            } else if let Some(phone) = connection.as_mut() {
                phone.command(LeSetAdvEnable::new(false)).await?;
            }
            let phone = connection
                .as_mut()
                .context("Phone HCI connection missing")?;
            let mut data = [0; 31];
            // Flags: LE General Discoverable + BR/EDR unsupported.
            // Service Data: AD length 23, type 0x16, little-endian FFF9, 20-byte EID.
            data[..7].copy_from_slice(&[2, 1, 6, 23, 0x16, 0xf9, 0xff]);
            data[7..27].copy_from_slice(&service_data);
            phone.command(LeSetAdvData::new(27, data)).await?;
            phone.command(LeSetAdvEnable::new(true)).await?;
            tracing::trace!(address = %self.address, "virtual Phone advertising updated");
            Ok::<(), anyhow::Error>(())
        })
        .await
        .context("virtual Phone HCI operation timed out")
        .and_then(|result| result);
        if result.is_err() {
            // Never reuse a partially configured or timed-out HCI stream.
            *connection = None;
        }
        result
    }
}

/// Sequential H4 command transport for btvirt's socket controller.
/// Command encoding and event parsing belong to bt-hci; this only frames TCP.
struct HciConnection(TcpStream);

impl HciConnection {
    async fn connect(address: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(address)
            .await
            .context("connect to btvirt Phone HCI server")?;
        stream.set_nodelay(true)?;
        Ok(Self(stream))
    }

    async fn command<C: SyncCmd<Return = ()>>(&mut self, command: C) -> Result<()> {
        let mut packet = vec![0; 1 + WriteHci::size(&command)];
        packet[0] = 0x01; // H4 Command packet.
        WriteHci::write_hci(&command, &mut packet[1..])
            .map_err(|error| anyhow::anyhow!("encode HCI command: {error:?}"))?;
        let opcode = C::OPCODE;
        tracing::trace!(opcode = opcode.to_raw(), "send virtual Phone HCI command");
        self.0.write_all(&packet).await?;
        loop {
            let mut header = [0; 3];
            self.0.read_exact(&mut header).await?;
            ensure!(
                header[0] == 0x04,
                "expected H4 Event packet, got {:#04x}",
                header[0]
            );
            let mut event = vec![0; 2 + usize::from(header[2])];
            event[..2].copy_from_slice(&header[1..]);
            self.0.read_exact(&mut event[2..]).await?;
            let event = Event::from_hci_bytes_complete(&event)
                .map_err(|error| anyhow::anyhow!("decode HCI event: {error:?}"))?;
            tracing::trace!(?event, "receive virtual Phone HCI event");
            match event {
                Event::CommandComplete(completed) if completed.cmd_opcode == opcode => {
                    let completed = CommandCompleteWithStatus::try_from(completed)
                        .map_err(|error| anyhow::anyhow!("decode HCI command status: {error:?}"))?;
                    return completed.to_result::<C>().map_err(|error| {
                        anyhow::anyhow!("HCI command {:#06x} failed: {error:?}", opcode.to_raw())
                    });
                }
                Event::CommandStatus(status) if status.cmd_opcode == opcode => {
                    if let Err(error) = status.status.to_result() {
                        bail!("HCI command {:#06x} failed: {error:?}", opcode.to_raw());
                    }
                }
                _ => {}
            }
        }
    }
}
