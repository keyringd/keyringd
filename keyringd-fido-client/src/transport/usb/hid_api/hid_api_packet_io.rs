use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::report::{HidReportLayout, parse_report_layout};
use super::hid_api_io_error::HidApiIoError;
use super::{hid_api_runtime::HidApiRuntime, owned_hid_device::OwnedHidDevice};
use crate::usb::{HidPacketIo, UsbHidError};

const READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Packet I/O implementations that hold the open HiDAPI device and HID report configuration.
/// Add/remove the report ID and pass only CTAPHID packets to the top.
pub struct HidApiPacketIo {
    device: Arc<Mutex<OwnedHidDevice>>,
    layout: HidReportLayout,
    path: std::ffi::CString,
}

impl HidApiPacketIo {
    pub(super) fn open(
        api: &hidapi::HidApi,
        path: &std::ffi::CStr,
        owner: &'static HidApiRuntime,
    ) -> Result<Self, HidApiIoError> {
        let device = api.open_path(path).map_err(HidApiIoError::Open)?;
        let mut report_descriptor = vec![0; hidapi::MAX_REPORT_DESCRIPTOR_SIZE];
        let descriptor_length = device
            .get_report_descriptor(&mut report_descriptor)
            .map_err(HidApiIoError::ReadReportDescriptor)?;
        report_descriptor.truncate(descriptor_length);
        let layout = parse_report_layout(&report_descriptor)
            .map_err(HidApiIoError::InvalidReportDescriptor)?;
        Ok(Self {
            device: Arc::new(Mutex::new(OwnedHidDevice::new(device, owner))),
            layout,
            path: path.to_owned(),
        })
    }
}

impl HidPacketIo for HidApiPacketIo {
    type Error = UsbHidError;

    fn input_packet_size(&self) -> usize {
        self.layout.input_packet_size()
    }

    fn output_packet_size(&self) -> usize {
        self.layout.output_packet_size()
    }

    fn write_packet<'a>(
        &'a mut self,
        packet: &'a [u8],
    ) -> futures_util::future::BoxFuture<'a, Result<(), Self::Error>> {
        Box::pin(async move {
            match self.write_packet(packet).await {
                Ok(()) => Ok(()),
                Err(error) => Err(self.classify_error(error).await),
            }
        })
    }

    fn read_packet<'a>(
        &'a mut self,
    ) -> futures_util::future::BoxFuture<'a, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move {
            match self.read_packet().await {
                Ok(packet) => Ok(packet),
                Err(error) => Err(self.classify_error(error).await),
            }
        })
    }
}

impl HidApiPacketIo {
    async fn classify_error(&self, error: HidApiIoError) -> UsbHidError {
        let device_missing = if matches!(error, HidApiIoError::Read(_) | HidApiIoError::Write(_)) {
            let path = self.path.clone();
            match HidApiRuntime::shared() {
                Ok(owner) => owner
                    .execute(move |api| {
                        api.refresh_devices().map_err(HidApiIoError::Enumerate)?;
                        Ok(!api
                            .device_list()
                            .any(|device| device.path() == path.as_c_str()))
                    })
                    .await
                    .unwrap_or_else(|check_error| {
                        tracing::debug!(?check_error, "could not verify HID disconnection");
                        false
                    }),
                Err(_) => false,
            }
        } else {
            false
        };
        error.into_usb_error(device_missing)
    }

    async fn write_packet(&mut self, packet: &[u8]) -> Result<(), HidApiIoError> {
        if packet.len() != self.layout.output_packet_size() {
            return Err(HidApiIoError::InvalidReportLength {
                expected: self.layout.output_packet_size(),
                actual: packet.len(),
            });
        }
        crate::util::wire_trace::trace_bytes("ctaphid", "out", packet, None);
        let device = Arc::clone(&self.device);
        let packet = packet.to_vec();
        let report_id = self.layout.report_id();
        tokio::task::spawn_blocking({
            let span = tracing::Span::current();
            move || {
                span.in_scope(|| {
                    let device = device.lock().map_err(|_| HidApiIoError::LockPoisoned)?;
                    let mut report = vec![0u8; packet.len() + 1];
                    report[0] = report_id;
                    report[1..].copy_from_slice(&packet);
                    let actual = device.raw().write(&report).map_err(HidApiIoError::Write)?;
                    if actual != report.len() {
                        return Err(HidApiIoError::ShortWrite {
                            expected: report.len(),
                            actual,
                        });
                    }
                    Ok(())
                })
            }
        })
        .await
        .map_err(HidApiIoError::Task)?
    }
    async fn read_packet(&mut self) -> Result<Vec<u8>, HidApiIoError> {
        let device = Arc::clone(&self.device);
        let layout = self.layout;
        tokio::task::spawn_blocking({
            let span = tracing::Span::current();
            move || {
                span.in_scope(|| {
                    let device = device.lock().map_err(|_| HidApiIoError::LockPoisoned)?;
                    let report_id_length = usize::from(layout.uses_report_ids());
                    let mut report = vec![0u8; layout.input_packet_size() + report_id_length];
                    let actual = device
                        .raw()
                        .read_timeout(&mut report, READ_TIMEOUT.as_millis() as i32)
                        .map_err(HidApiIoError::Read)?;
                    if actual == 0 {
                        return Err(HidApiIoError::ReadTimedOut);
                    }
                    if actual != report.len() {
                        return Err(HidApiIoError::InvalidReportLength {
                            expected: report.len(),
                            actual,
                        });
                    }
                    let packet = if layout.uses_report_ids() {
                        if report[0] != layout.report_id() {
                            return Err(HidApiIoError::InvalidReportId {
                                expected: layout.report_id(),
                                actual: report[0],
                            });
                        }
                        report.split_off(1)
                    } else {
                        report
                    };
                    crate::util::wire_trace::trace_bytes("ctaphid", "in", &packet, None);
                    Ok(packet)
                })
            }
        })
        .await
        .map_err(HidApiIoError::Task)?
    }
}
