//! Named Pipe Agent protocol observation; no UI or SSH server lifecycle.
use crate::environment::windows::TestApp;
use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub fn agent_identities(app: &TestApp) -> Result<Vec<String>> {
    let pipe = app.agent_pipe()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut stream = loop {
        match OpenOptions::new().read(true).write(true).open(&pipe) {
            Ok(stream) => break stream,
            Err(error) if Instant::now() < deadline => {
                if error.raw_os_error() != Some(2) && error.raw_os_error() != Some(231) {
                    return Err(error.into());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error.into()),
        }
    };
    stream.write_all(&[0, 0, 0, 1, 11])?;
    let mut length = [0; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    ensure!(length <= 1024 * 1024, "Agent response too large");
    let mut response = vec![0; length];
    stream.read_exact(&mut response)?;
    ensure!(
        response.first() == Some(&12),
        "Agent did not return identities"
    );
    let mut remaining = &response[1..];
    let count = take_u32(&mut remaining)?;
    ensure!(count < 1000, "invalid identity count");
    let mut identities = Vec::new();
    for _ in 0..count {
        let blob = take_string(&mut remaining)?;
        let mut fields = blob;
        let algorithm = std::str::from_utf8(take_string(&mut fields)?)?;
        let comment = std::str::from_utf8(take_string(&mut remaining)?)?;
        identities.push(format!("{algorithm} {} {comment}", STANDARD.encode(blob)));
    }
    ensure!(remaining.is_empty(), "trailing identity response data");
    Ok(identities)
}
fn take_u32(bytes: &mut &[u8]) -> Result<u32> {
    ensure!(bytes.len() >= 4, "truncated Agent response");
    let result = u32::from_be_bytes(bytes[..4].try_into()?);
    *bytes = &bytes[4..];
    Ok(result)
}
fn take_string<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8]> {
    let length = take_u32(bytes)? as usize;
    ensure!(bytes.len() >= length, "truncated Agent string");
    let (result, rest) = bytes.split_at(length);
    *bytes = rest;
    Ok(result)
}

pub fn agent_identity(
    app: &TestApp,
    kind: keyringd_core::model::ProfileSshIdentityKind,
) -> Result<String> {
    use keyringd_core::model::ProfileSshIdentityKind;
    let comment = match kind {
        ProfileSshIdentityKind::DeviceCertificate => " device certificate".to_owned(),
        ProfileSshIdentityKind::DeviceKey => format!(
            " device key@{}",
            gethostname::gethostname().to_string_lossy()
        ),
        ProfileSshIdentityKind::Passkey => " passkey".to_owned(),
    };
    agent_identities(app)?
        .into_iter()
        .find(|identity| identity.ends_with(&comment))
        .ok_or_else(|| anyhow::anyhow!("Agent did not publish {kind:?}"))
}
