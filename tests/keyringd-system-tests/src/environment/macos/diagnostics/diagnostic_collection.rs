use anyhow::{Context, Result, ensure};
use keyringd_testkit::macos_ui_protocol::{
    ControlCommand, ControlResponse, Scope, UiCommand, UiResponse,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

pub(crate) fn exchange(
    address: SocketAddr,
    command: &ControlCommand,
    timeout: Duration,
) -> Result<ControlResponse> {
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    writeln!(stream, "{}", serde_json::to_string(command)?)?;
    let mut response = String::new();
    let received = BufReader::new(stream)
        .take(8 * 1024 * 1024)
        .read_line(&mut response)?;
    ensure!(
        received > 0,
        "fixture closed the control connection without a response"
    );
    let response: ControlResponse =
        serde_json::from_str(&response).context("invalid control response")?;
    if let ControlResponse::Error { message } = &response {
        anyhow::bail!("control command failed: {message}");
    }
    Ok(response)
}

pub fn collect(directory: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for file in ["app.log", "profiles.json", "setting.json"] {
        let source = directory.join(file);
        if source.is_file() {
            std::fs::copy(source, destination.join(file))?;
        }
    }
    let rpc = |ui| -> Result<UiResponse> {
        let address = std::fs::read_to_string(directory.join("control-address"))?
            .trim()
            .parse()?;
        match exchange(
            address,
            &ControlCommand {
                ui: Some(ui),
                ..Default::default()
            },
            Duration::from_millis(750),
        )? {
            ControlResponse::Ui { result } => Ok(result),
            other => anyhow::bail!("expected diagnostic UI response: {other:?}"),
        }
    };
    let tree = match rpc(UiCommand::Tree) {
        Ok(UiResponse::Tree { tree }) => tree,
        result => format!("AX tree unavailable: {result:?}"),
    };
    std::fs::write(destination.join("ax-tree.txt"), tree)?;
    let path = destination.join("window.png");
    match rpc(UiCommand::Screenshot {
        path: path.clone(),
        scope: Scope::KeyWindow,
    }) {
        Ok(UiResponse::Screenshot { path: written }) if written == path && path.is_file() => {}
        result => {
            std::fs::write(
                destination.join("screenshot-error.txt"),
                format!("Screenshot unavailable: {result:?}"),
            )?;
        }
    }
    Ok(())
}
