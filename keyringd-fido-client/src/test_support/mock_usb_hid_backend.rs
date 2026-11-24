use futures_util::future::BoxFuture;
use keyringd_fido_client::backend::UsbHidBackend;
use keyringd_fido_client::usb::{HidPacketIo, UsbHidDeviceId, UsbHidDeviceInfo, UsbHidError};

mockall::mock! {
    /// A mock that sets the enumeration and connection of USB devices for each test.
    pub(crate) UsbHidBackend {}
    impl UsbHidBackend for UsbHidBackend {
        fn enumerate(&self) -> BoxFuture<'static, Result<Vec<UsbHidDeviceInfo>, UsbHidError>>;
        fn open<'a>(&'a self, id: &'a UsbHidDeviceId)
            -> BoxFuture<'static, Result<Box<dyn HidPacketIo<Error = UsbHidError>>, UsbHidError>>;
    }
}

pub(crate) fn empty_usb_backend() -> MockUsbHidBackend {
    let mut backend = MockUsbHidBackend::new();
    backend
        .expect_enumerate()
        .returning(|| Box::pin(async { Ok(Vec::new()) }));
    backend.expect_open().never();
    backend
}
