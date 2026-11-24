use super::usb_ip_pending_transfer::UsbIpPendingTransfer;
use crate::usbip_hid_backend::protocol::transfer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub(crate) struct UsbIpTransferGuard {
    pub sequence: u32,
    pub device: u32,
    pub pending: Arc<Mutex<HashMap<u32, UsbIpPendingTransfer>>>,
    pub unlinks: Arc<Mutex<HashMap<u32, u32>>>,
    pub next_sequence: Arc<AtomicU32>,
    pub outgoing: mpsc::UnboundedSender<Vec<u8>>,
    pub completed: bool,
}
impl Drop for UsbIpTransferGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let mut pending = self.pending.lock().unwrap();
        let Some(transfer) = pending.get_mut(&self.sequence) else {
            return;
        };
        // Keep the framing metadata until RET_SUBMIT or successful RET_UNLINK.
        // A transfer can finish before the server processes our unlink request.
        transfer.response.take();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        self.unlinks.lock().unwrap().insert(sequence, self.sequence);
        let mut header = transfer::transfer_header(2, sequence, self.device, false, 0);
        header[20..24].copy_from_slice(&self.sequence.to_be_bytes());
        let _ = self.outgoing.send(header.to_vec());
    }
}
