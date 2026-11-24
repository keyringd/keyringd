use super::AuthenticatorServerError;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) async fn request(
    address: SocketAddr,
    timeout: Duration,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Vec<u8>, AuthenticatorServerError> {
    tokio::time::timeout(timeout, async {
        let mut stream = TcpStream::connect(address).await?;
        // HTTP/1.0 requests keep responses unchunked and let the server close the connection.
        let headers = format!("{method} {path} HTTP/1.0\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
        stream.write_all(headers.as_bytes()).await?;
        stream.write_all(body).await?;
        let mut response = Vec::new();
        (&mut stream).take(1_048_577).read_to_end(&mut response).await?;
        if response.len() > 1_048_576 {
            return Err(AuthenticatorServerError::Protocol("response exceeds 1 MiB".into()));
        }
        parse_response(&response)
    }).await.map_err(|_| AuthenticatorServerError::TimedOut)?
}

fn parse_response(response: &[u8]) -> Result<Vec<u8>, AuthenticatorServerError> {
    let end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| AuthenticatorServerError::Protocol("missing HTTP headers".into()))?;
    let headers = std::str::from_utf8(&response[..end])
        .map_err(|_| AuthenticatorServerError::Protocol("invalid HTTP headers".into()))?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| AuthenticatorServerError::Protocol("invalid HTTP status".into()))?;
    let body = response[end + 4..].to_vec();
    if !(200..300).contains(&status) {
        return Err(AuthenticatorServerError::Http {
            status,
            message: String::from_utf8_lossy(&body).into_owned(),
        });
    }
    if let Some(length) = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    {
        let length = length
            .1
            .trim()
            .parse::<usize>()
            .map_err(|_| AuthenticatorServerError::Protocol("invalid content length".into()))?;
        if body.len() != length {
            return Err(AuthenticatorServerError::Protocol(
                "truncated HTTP response".into(),
            ));
        }
    }
    Ok(body)
}
