use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) fn append_string(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(
        &u32::try_from(value.len())
            .expect("test SSH string length fits in u32")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(value);
}

pub(crate) fn unsupported_public_key(algorithm: &[u8], data: &[u8]) -> Vec<u8> {
    let mut key = Vec::new();
    append_string(&mut key, algorithm);
    append_string(&mut key, data);
    key
}

pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(4 + payload.len());
    framed.extend_from_slice(
        &u32::try_from(payload.len())
            .expect("test frame length fits in u32")
            .to_be_bytes(),
    );
    framed.extend_from_slice(payload);
    framed
}

pub(crate) async fn write_frame<W>(writer: &mut W, payload: &[u8])
where
    W: AsyncWrite + Unpin,
{
    writer.write_all(&frame(payload)).await.unwrap();
}

pub(crate) async fn read_frame<R>(reader: &mut R) -> Vec<u8>
where
    R: AsyncRead + Unpin,
{
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length).await.unwrap();
    let mut payload = vec![0_u8; u32::from_be_bytes(length) as usize];
    reader.read_exact(&mut payload).await.unwrap();
    payload
}
