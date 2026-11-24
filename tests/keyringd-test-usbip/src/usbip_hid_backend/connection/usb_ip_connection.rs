use super::{
    usb_ip_pending_transfer::UsbIpPendingTransfer, usb_ip_transfer_guard::UsbIpTransferGuard,
    usb_ip_transfer_result::UsbIpTransferResult,
};
use crate::usbip_hid_backend::{
    UsbIpError,
    protocol::{UsbIpDeviceRecord, management, transfer},
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

pub(crate) struct UsbIpConnection {
    device: u32,
    closed: Arc<AtomicBool>,
    next_sequence: Arc<AtomicU32>,
    pending: Arc<Mutex<HashMap<u32, UsbIpPendingTransfer>>>,
    unlinks: Arc<Mutex<HashMap<u32, u32>>>,
    outgoing: mpsc::UnboundedSender<Vec<u8>>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}
impl UsbIpConnection {
    pub(crate) async fn import(device: &UsbIpDeviceRecord) -> Result<Self, UsbIpError> {
        let mut stream = TcpStream::connect(device.server).await?;
        stream.set_nodelay(true)?;
        let mut request = management::management_header(0x8003).to_vec();
        let mut bus = [0; 32];
        bus[..device.bus_id.len()].copy_from_slice(device.bus_id.as_bytes());
        request.extend_from_slice(&bus);
        stream.write_all(&request).await?;
        management::response(&mut stream, 0x0003).await?;
        let mut record = [0; 312];
        stream.read_exact(&mut record).await?;
        let imported = UsbIpDeviceRecord::parse(device.server, &record)?;
        if imported.bus_id != device.bus_id {
            return Err(UsbIpError::Protocol("imported bus ID changed"));
        }
        let (mut read, mut write) = stream.into_split();
        let pending = Arc::new(Mutex::new(HashMap::<u32, UsbIpPendingTransfer>::new()));
        let unlinks = Arc::new(Mutex::new(HashMap::<u32, u32>::new()));
        let (outgoing, mut outgoing_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let closed = Arc::new(AtomicBool::new(false));
        let reader_closed = Arc::clone(&closed);
        let writer_closed = Arc::clone(&closed);
        let writer_pending = Arc::clone(&pending);
        let reader_pending = Arc::clone(&pending);
        let reader_unlinks = Arc::clone(&unlinks);
        // A persistent reader preserves response framing when an individual
        // read future is dropped to send CTAPHID_CANCEL or to select another key.
        let reader = tokio::spawn(async move {
            let result: Result<(), UsbIpError> = async {
                loop {
                    let mut h = [0; 48];
                    read.read_exact(&mut h).await?;
                    let command = u32::from_be_bytes(h[..4].try_into().unwrap());
                    let sequence = u32::from_be_bytes(h[4..8].try_into().unwrap());
                    if command == 4 {
                        let original = reader_unlinks.lock().unwrap().remove(&sequence);
                        let status = i32::from_be_bytes(h[20..24].try_into().unwrap());
                        if matches!(status, 0 | -104)
                            && let Some(original) = original
                        {
                            reader_pending.lock().unwrap().remove(&original);
                        }
                        continue;
                    }
                    if command != 3 {
                        return Err(UsbIpError::Protocol("unexpected transfer response"));
                    }
                    let transfer = reader_pending
                        .lock()
                        .unwrap()
                        .remove(&sequence)
                        .ok_or(UsbIpError::Protocol("unknown transfer sequence"))?;
                    let iso_packets = u32::from_be_bytes(h[32..36].try_into().unwrap());
                    if !matches!(iso_packets, 0 | u32::MAX) {
                        return Err(UsbIpError::Protocol(
                            "isochronous responses are unsupported",
                        ));
                    }
                    let length = u32::from_be_bytes(h[24..28].try_into().unwrap()) as usize;
                    if length > transfer.maximum || length > transfer::MAX_TRANSFER {
                        return Err(UsbIpError::Protocol("transfer response exceeds request"));
                    }
                    let mut data = if transfer.input {
                        vec![0; length]
                    } else {
                        Vec::new()
                    };
                    read.read_exact(&mut data).await?;
                    let status = i32::from_be_bytes(h[20..24].try_into().unwrap());
                    if let Some(sender) = transfer.response {
                        let _ = sender.send(if status == 0 {
                            Ok(UsbIpTransferResult {
                                actual_length: length,
                                data,
                            })
                        } else {
                            Err(UsbIpError::Status(status))
                        });
                    }
                }
            }
            .await;
            let description = result
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "connection closed".into());
            fail_pending(&reader_pending, &reader_closed, &description);
        });
        let writer = tokio::spawn(async move {
            while let Some(message) = outgoing_rx.recv().await {
                if let Err(error) = write.write_all(&message).await {
                    fail_pending(&writer_pending, &writer_closed, &error.to_string());
                    break;
                }
            }
        });
        Ok(Self {
            device: imported.device_id,
            closed,
            next_sequence: Arc::new(AtomicU32::new(1)),
            pending,
            unlinks,
            outgoing,
            reader,
            writer,
        })
    }
    pub(crate) async fn submit(
        &self,
        input: bool,
        endpoint: u8,
        length: usize,
        setup: [u8; 8],
        payload: &[u8],
        interval: u32,
    ) -> Result<UsbIpTransferResult, UsbIpError> {
        if length > transfer::MAX_TRANSFER || (!input && payload.len() != length) {
            return Err(UsbIpError::Protocol("invalid transfer length"));
        }
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if self.closed.load(Ordering::Acquire) {
                return Err(UsbIpError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "USB/IP connection closed",
                )));
            }
            pending.insert(
                sequence,
                UsbIpPendingTransfer {
                    input,
                    maximum: length,
                    response: Some(sender),
                },
            );
        }
        let mut guard = UsbIpTransferGuard {
            sequence,
            device: self.device,
            pending: Arc::clone(&self.pending),
            unlinks: Arc::clone(&self.unlinks),
            next_sequence: Arc::clone(&self.next_sequence),
            outgoing: self.outgoing.clone(),
            completed: false,
        };
        let mut h = transfer::transfer_header(1, sequence, self.device, input, endpoint);
        h[24..28].copy_from_slice(&(length as u32).to_be_bytes());
        h[32..36].copy_from_slice(&u32::MAX.to_be_bytes());
        h[36..40].copy_from_slice(&interval.to_be_bytes());
        h[40..48].copy_from_slice(&setup);
        let mut message = h.to_vec();
        if !input {
            message.extend_from_slice(payload);
        }
        self.outgoing
            .send(message)
            .map_err(|_| UsbIpError::Protocol("USB/IP writer stopped"))?;
        let result = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .map_err(|_| UsbIpError::TimedOut)?
            .map_err(|_| UsbIpError::Protocol("USB/IP reader stopped"))?;
        guard.completed = true;
        result
    }
    pub(crate) async fn descriptor(
        &self,
        descriptor_type: u8,
        index: u16,
        length: u16,
    ) -> Result<Vec<u8>, UsbIpError> {
        let mut setup = [0; 8];
        setup[0] = if descriptor_type == 0x22 { 0x81 } else { 0x80 };
        setup[1] = 6;
        setup[2..4].copy_from_slice(&(u16::from(descriptor_type) << 8).to_le_bytes());
        setup[4..6].copy_from_slice(&index.to_le_bytes());
        setup[6..8].copy_from_slice(&length.to_le_bytes());
        Ok(self
            .submit(true, 0, usize::from(length), setup, &[], 0)
            .await?
            .data)
    }
    pub(crate) async fn configure(&self, configuration: u8) -> Result<(), UsbIpError> {
        let mut setup = [0; 8];
        setup[1] = 9;
        setup[2] = configuration;
        self.submit(false, 0, 0, setup, &[], 0).await?;
        Ok(())
    }
}
// Mark closure and drain under the insertion lock so a concurrent submission
// either joins the failed requests or observes closure before registering itself.
fn fail_pending(
    pending: &Mutex<HashMap<u32, UsbIpPendingTransfer>>,
    closed: &AtomicBool,
    description: &str,
) {
    let mut pending = pending.lock().unwrap();
    closed.store(true, Ordering::Release);
    for (_, transfer) in pending.drain() {
        if let Some(sender) = transfer.response {
            let _ = sender.send(Err(UsbIpError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                description.to_owned(),
            ))));
        }
    }
}

impl Drop for UsbIpConnection {
    fn drop(&mut self) {
        self.reader.abort();
        self.writer.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::test_support::{connect, reply, request};
    use super::*;
    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Input responses are dispatched by sequence despite reordering and fragmentation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "correlate fragmented and reordered input responses with their transfer sequence" {
                    let (connection, mut server) = connect().await;
                    let responses = async {
                        let first = request(&mut server).await;
                        let second = request(&mut server).await;
                        reply(&mut server, &second, 3, &[2; 4], 0).await;
                        reply(&mut server, &first, 3, &[1; 4], 0).await;
                    };
                    let (first, second, ()) = tokio::join!(
                        connection.submit(true, 1, 4, [0; 8], &[], 1),
                        connection.submit(true, 2, 4, [0; 8], &[], 1),
                        responses
                    );
                    assert_eq!(first.unwrap().data, [1; 4]);
                    assert_eq!(second.unwrap().data, [2; 4]);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Cancel racing a completed input does not corrupt the next transfer" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "keep the next transfer readable after cancellation overlaps input completion" {
                    let (connection, mut server) = connect().await;
                    let (result, original) = tokio::join!(
                        tokio::time::timeout(
                            Duration::from_millis(20),
                            connection.submit(true, 1, 4, [0; 8], &[], 1)
                        ),
                        request(&mut server)
                    );
                    assert!(result.is_err());
                    let unlink = request(&mut server).await;
                    assert_eq!(u32::from_be_bytes(unlink[..4].try_into().unwrap()), 2);
                    assert_eq!(&unlink[20..24], &original[4..8]);
                    let responses = async {
                        // Completion won the race. The cancelled future's input payload still needs draining.
                        reply(&mut server, &original, 3, &[9; 4], 0).await;
                        reply(&mut server, &unlink, 4, &[], -2).await;
                        let next = request(&mut server).await;
                        reply(&mut server, &next, 3, &[7; 4], 0).await;
                    };
                    let (result, ()) = tokio::join!(connection.submit(true, 1, 4, [0; 8], &[], 1), responses);
                    assert_eq!(result.unwrap().data, [7; 4]);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Successful unlink without submit response allows the next transfer" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "allow a subsequent transfer after confirmed unlink without a submit reply" {
                    let (connection, mut server) = connect().await;
                    let (result, _) = tokio::join!(
                        tokio::time::timeout(
                            Duration::from_millis(20),
                            connection.submit(true, 1, 4, [0; 8], &[], 1)
                        ),
                        request(&mut server)
                    );
                    assert!(result.is_err());
                    let unlink = request(&mut server).await;
                    let responses = async {
                        reply(&mut server, &unlink, 4, &[], -104).await;
                        let next = request(&mut server).await;
                        reply(&mut server, &next, 3, &[7; 4], 0).await;
                    };
                    let (result, ()) = tokio::join!(connection.submit(true, 1, 4, [0; 8], &[], 1), responses);
                    assert_eq!(result.unwrap().data, [7; 4]);
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Oversized response fails waiters without allocating advertised payload" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reject an oversized transfer response before allocating its declared payload" {
                    let (connection, mut server) = connect().await;
                    let responses = async {
                        let requested = request(&mut server).await;
                        let mut response = [0; 48];
                        response[..4].copy_from_slice(&3u32.to_be_bytes());
                        response[4..8].copy_from_slice(&requested[4..8]);
                        response[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
                        server.write_all(&response).await.unwrap();
                    };
                    let (result, ()) = tokio::join!(connection.submit(true, 1, 4, [0; 8], &[], 1), responses);
                    assert!(result.is_err());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Disconnect fails pending transfers promptly" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "release outstanding request futures as soon as the connection is lost" {
                    let (connection, mut server) = connect().await;
                    let response = async {
                        request(&mut server).await;
                        drop(server);
                    };
                    let (result, ()) = tokio::join!(connection.submit(true, 1, 4, [0; 8], &[], 1), response);
                    assert!(matches!(result, Err(UsbIpError::Io(_))));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbIpConnection::submit" {
            case "Requests after an idle disconnect fail without waiting for timeout" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "return a disconnected error immediately when submitting after idle EOF" {
                    let (mut connection, server) = connect().await;
                    drop(server);
                    tokio::time::timeout(Duration::from_secs(1), &mut connection.reader)
                        .await
                        .unwrap()
                        .unwrap();
                    for input in [true, false] {
                        let result = tokio::time::timeout(
                            Duration::from_millis(200),
                            connection.submit(input, 1, 0, [0; 8], &[], 1),
                        )
                        .await
                        .unwrap();
                        assert!(matches!(result, Err(UsbIpError::Io(_))));
                    }
                }
            }
        }
    }
}
