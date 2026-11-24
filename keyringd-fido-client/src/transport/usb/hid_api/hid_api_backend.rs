use super::hid_api_runtime::HidApiRuntime;
use super::{HidApiIoError, HidApiPacketIo};
use crate::backend::UsbHidBackend;
use crate::usb::{HidPacketIo, UsbHidDeviceId, UsbHidDeviceInfo, UsbHidError};
use futures_util::future::BoxFuture;

/// USB backend using HiDAPI. Responsible for enumerating FIDO HID devices and opening a connection for a specified path.
#[derive(Clone, Copy, Debug, Default)]
pub struct HidApiBackend;

impl UsbHidBackend for HidApiBackend {
    fn enumerate(&self) -> BoxFuture<'_, Result<Vec<UsbHidDeviceInfo>, UsbHidError>> {
        Box::pin(async {
            HidApiRuntime::shared()
                .map_err(UsbHidError::new)?
                .execute(|api| {
                    api.refresh_devices().map_err(HidApiIoError::Enumerate)?;
                    Ok(api
                        .device_list()
                        .filter(|device| {
                            super::is_ctap_hid_device(device.usage_page(), device.usage())
                        })
                        .map(|device| UsbHidDeviceInfo {
                            id: UsbHidDeviceId::new(device.path().to_bytes().to_vec()),
                            vendor_id: device.vendor_id(),
                            product_id: device.product_id(),
                            manufacturer: device.manufacturer_string().map(str::to_owned),
                            product: device.product_string().map(str::to_owned),
                            serial_number: device.serial_number().map(str::to_owned),
                        })
                        .collect())
                })
                .await
                .map_err(UsbHidError::new)
        })
    }

    fn open<'a>(
        &'a self,
        id: &'a UsbHidDeviceId,
    ) -> BoxFuture<'a, Result<Box<dyn HidPacketIo<Error = UsbHidError>>, UsbHidError>> {
        Box::pin(async move {
            let path = std::ffi::CString::new(id.as_bytes()).map_err(UsbHidError::new)?;
            let runtime = HidApiRuntime::shared().map_err(UsbHidError::new)?;
            let io = runtime
                .execute(move |api| HidApiPacketIo::open(api, &path, runtime))
                .await
                .map_err(UsbHidError::new)?;
            Ok(Box::new(io) as Box<dyn HidPacketIo<Error = UsbHidError>>)
        })
    }
}
