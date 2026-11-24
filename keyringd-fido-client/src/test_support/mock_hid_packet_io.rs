use crate::usb::HidPacketIo;
use futures_util::future::BoxFuture;

mockall::mock! {
    /// A mock that sets the response, disconnection, and wait of HID packets for each test.
    pub(crate) HidPacketIo {}
    impl HidPacketIo for HidPacketIo {
        type Error = std::io::Error;
        fn input_packet_size(&self) -> usize;
        fn output_packet_size(&self) -> usize;
        fn write_packet<'a>(&'a mut self, packet: &'a [u8]) -> BoxFuture<'static, Result<(), std::io::Error>>;
        fn read_packet(&mut self) -> BoxFuture<'static, Result<Vec<u8>, std::io::Error>>;
    }
}
