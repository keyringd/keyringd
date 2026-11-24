use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use keyringd_test_usbip::authenticator_server::client::AuthenticatorServerClient;
use serde_json::{Value, json};

use super::UsbIpAttachment;
use crate::environment::testbed::{ContainerCommand, TestbedExecutionRequest};

/// Privileged USB/IP imports owned by the case's live fixture connection.
/// The authenticator itself remains owned by the unprivileged common fixture.
pub(crate) struct UsbIpBroker {
    socket: PathBuf,
    _directory: tempfile::TempDir,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl UsbIpBroker {
    pub(crate) fn start(request: &TestbedExecutionRequest) -> Result<Self> {
        crate::util::command::require_success(
            "USB HID kernel module load",
            "modprobe",
            ["usbhid"],
            Path::new("/"),
        )?;
        let directory = tempfile::Builder::new()
            .prefix("keyringd-usb-")
            .tempdir_in("/run")?;
        let socket = directory.path().join("control.sock");
        let listener =
            UnixListener::bind(&socket).context("bind privileged USB/IP control socket")?;
        std::os::unix::fs::chown(&socket, Some(request.owner_uid), Some(request.owner_gid))?;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let request = request.clone();
        let thread = thread::spawn(move || {
            let imports = Arc::new(Mutex::new(Vec::new()));
            let pending_removals = Arc::new(Mutex::new(Vec::new()));
            let mut sessions = Vec::new();
            while !thread_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let stop = Arc::clone(&thread_stop);
                        let imports = Arc::clone(&imports);
                        let pending_removals = Arc::clone(&pending_removals);
                        let request = request.clone();
                        sessions.push(thread::spawn(move || {
                            serve(stream, &request, &stop, &imports, &pending_removals)
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            let mut first_error = None;
            for session in sessions {
                match session.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        first_error.get_or_insert(error);
                    }
                    Err(_) => {
                        first_error
                            .get_or_insert_with(|| anyhow::anyhow!("USB/IP lease thread panicked"));
                    }
                }
            }
            let mut orphaned = imports
                .lock()
                .map_err(|_| anyhow::anyhow!("USB/IP import lock poisoned"))?;
            for orphan in orphaned.drain(..) {
                let mut orphan = Some(orphan);
                if let Err(error) = disconnect(&mut orphan, &request) {
                    eprintln!("Final USB/IP lease cleanup: {error:#}");
                    first_error.get_or_insert(error);
                }
            }
            first_error.map_or(Ok(()), Err)
        });
        Ok(Self {
            socket,
            _directory: directory,
            stop,
            thread: Some(thread),
        })
    }

    pub(crate) fn socket(&self) -> &Path {
        &self.socket
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::SeqCst);
        let joined = self
            .thread
            .take()
            .map(|thread| {
                thread
                    .join()
                    .map_err(|_| anyhow::anyhow!("USB/IP broker thread panicked"))?
            })
            .unwrap_or(Ok(()));
        let removed = std::fs::remove_file(&self.socket);
        joined?;
        if let Err(error) = removed {
            ensure!(
                error.kind() == std::io::ErrorKind::NotFound,
                "remove USB/IP control socket: {error}"
            );
        }
        Ok(())
    }
}

impl Drop for UsbIpBroker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn serve(
    stream: UnixStream,
    request: &TestbedExecutionRequest,
    stop: &AtomicBool,
    imports: &Mutex<Vec<(UsbIpAttachment, bool)>>,
    pending_removals: &Arc<Mutex<Vec<PathBuf>>>,
) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut stream = BufReader::new(stream);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = AuthenticatorServerClient::from_endpoint(&request.authenticator_control_url)?;
    let mut owner = None;
    let mut attachment = None;
    let mut line = String::new();
    let result = (|| -> Result<()> {
        while !stop.load(Ordering::SeqCst) {
            match stream.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) if !line.ends_with('\n') => continue,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
            let response = (|| -> Result<Value> {
                let command: Value = serde_json::from_str(&line)?;
                let id = command["id"]
                    .as_str()
                    .context("USB/IP request is missing authenticator id")?;
                let key = client.authenticator(id)?;
                if let Some(owner) = &owner {
                    ensure!(
                        owner == id,
                        "a USB/IP lease cannot change authenticator identity"
                    );
                } else {
                    owner = Some(id.to_owned());
                }
                let mut orphaned = imports
                    .lock()
                    .map_err(|_| anyhow::anyhow!("USB/IP import lock poisoned"))?;
                match command["op"].as_str() {
                    Some("attach") => {
                        while let Some(orphan) = orphaned.pop() {
                            let mut orphan = Some(orphan);
                            if let Err(error) = disconnect(&mut orphan, request) {
                                orphaned.push(
                                    orphan.expect("failed cleanup retains its owned attachment"),
                                );
                                return Err(
                                    error.context("previous USB/IP lease could not be unbound")
                                );
                            }
                        }
                        if attachment.is_none() {
                            let info = runtime.block_on(key.info())?;
                            let imported = UsbIpAttachment::attach(
                                stop,
                                "127.0.0.1",
                                &info.bus_id,
                                Arc::clone(pending_removals),
                            )?;
                            imported.allow_e2e_user(request.owner_uid, request.owner_gid)?;
                            crate::util::command::require_success(
                                "dynamic USB authenticator bind",
                                "machinectl",
                                vec![
                                    "--mkdir".to_owned(),
                                    "bind".to_owned(),
                                    request.machine.clone(),
                                    imported.hidraw().display().to_string(),
                                    imported.hidraw().display().to_string(),
                                ],
                                Path::new("/"),
                            )?;
                            attachment = Some((imported, true));
                        }
                        Ok(json!({"hidraw": attachment.as_ref().unwrap().0.hidraw()}))
                    }
                    Some("detach") => {
                        disconnect(&mut attachment, request)?;
                        Ok(json!({}))
                    }
                    _ => anyhow::bail!("unknown USB/IP operation"),
                }
            })();
            line.clear();
            let response = match response {
                Ok(value) => json!({"ok": value}),
                Err(error) => json!({"error": format!("{error:#}")}),
            };
            writeln!(stream.get_mut(), "{response}")?;
        }
        Ok(())
    })();
    let mut orphaned = imports
        .lock()
        .map_err(|_| anyhow::anyhow!("USB/IP import lock poisoned"))?;
    let cleanup = disconnect(&mut attachment, request);
    if let Some(attachment) = attachment.take() {
        orphaned.push(attachment);
    }
    result.and(cleanup)
}

fn disconnect(
    attachment: &mut Option<(UsbIpAttachment, bool)>,
    request: &TestbedExecutionRequest,
) -> Result<()> {
    let Some((imported, bound)) = attachment.as_mut() else {
        return Ok(());
    };
    if *bound {
        ContainerCommand::new(&request.machine, request.owner_uid, request.owner_gid)
            .run_as_root_with_timeout(
                "dynamic USB authenticator unbind",
                Duration::from_secs(5),
                [
                    "umount",
                    "--",
                    imported.hidraw().to_str().context("non-UTF8 hidraw path")?,
                ],
            )?;
        *bound = false;
    }
    imported.detach()?;
    *attachment = None;
    Ok(())
}
