use super::super::{
    connection::UsbIpConnection,
    protocol::{UsbIpDeviceRecord, management},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub(in crate::usbip_hid_backend) async fn connect() -> (UsbIpConnection, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut record = [0; 312];
    record[256..259].copy_from_slice(b"1-1");
    record[288..292].copy_from_slice(&1u32.to_be_bytes());
    record[292..296].copy_from_slice(&1u32.to_be_bytes());
    let device = UsbIpDeviceRecord::parse(listener.local_addr().unwrap(), &record).unwrap();
    let server = async {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 40];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(&request[..8], &management::management_header(0x8003));
        stream
            .write_all(&management::management_header(0x0003))
            .await
            .unwrap();
        stream.write_all(&record).await.unwrap();
        stream
    };
    let (connection, stream) = tokio::join!(UsbIpConnection::import(&device), server);
    (connection.unwrap(), stream)
}

pub(in crate::usbip_hid_backend) async fn request(stream: &mut TcpStream) -> [u8; 48] {
    let mut header = [0; 48];
    stream.read_exact(&mut header).await.unwrap();
    header
}

pub(in crate::usbip_hid_backend) async fn reply(
    stream: &mut TcpStream,
    request: &[u8; 48],
    command: u32,
    data: &[u8],
    status: i32,
) {
    let mut response = [0; 48];
    response[..4].copy_from_slice(&command.to_be_bytes());
    response[4..8].copy_from_slice(&request[4..8]);
    response[20..24].copy_from_slice(&status.to_be_bytes());
    response[24..28].copy_from_slice(&(data.len() as u32).to_be_bytes());
    // RET_SUBMIT direction is zero even for input. Framing uses saved request metadata.
    let mut message = response.to_vec();
    message.extend_from_slice(data);
    for fragment in message.chunks(3) {
        stream.write_all(fragment).await.unwrap();
        tokio::task::yield_now().await;
    }
}
