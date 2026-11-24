use futures_util::future::BoxFuture;
/// One open HID-connected interface that sends and receives complete CTAPHID packets, excluding report IDs.
/// Even if the Future of the read is discarded in the middle, it is necessary to be able to continue to send and communicate on the same connection.
pub trait HidPacketIo: Send {
    type Error: std::error::Error + Send + Sync + 'static;
    fn input_packet_size(&self) -> usize;
    fn output_packet_size(&self) -> usize;
    fn write_packet<'a>(&'a mut self, packet: &'a [u8]) -> BoxFuture<'a, Result<(), Self::Error>>;
    fn read_packet(&mut self) -> BoxFuture<'_, Result<Vec<u8>, Self::Error>>;
}
impl<I: HidPacketIo + ?Sized> HidPacketIo for Box<I> {
    type Error = I::Error;
    fn input_packet_size(&self) -> usize {
        (**self).input_packet_size()
    }
    fn output_packet_size(&self) -> usize {
        (**self).output_packet_size()
    }
    fn write_packet<'a>(&'a mut self, packet: &'a [u8]) -> BoxFuture<'a, Result<(), Self::Error>> {
        (**self).write_packet(packet)
    }
    fn read_packet(&mut self) -> BoxFuture<'_, Result<Vec<u8>, Self::Error>> {
        (**self).read_packet()
    }
}
