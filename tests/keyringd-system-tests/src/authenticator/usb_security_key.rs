use super::{AuthenticatorControl, UsbConnection};
use anyhow::Result;
use keyringd_test_usbip::authenticator_server::client::AuthenticatorHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// A case-owned key. Clones refer to the same credentials and OS attachment.
#[derive(Clone)]
pub struct UsbSecurityKey {
    state: Rc<UsbSecurityKeyState>,
}

struct UsbSecurityKeyState {
    authenticator: AuthenticatorControl,
    connection: RefCell<Option<Box<dyn UsbConnection>>>,
    connection_state: Cell<ConnectionState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connected,
    NeedsCleanup,
}

impl UsbSecurityKey {
    pub(crate) fn new(
        runtime: Arc<Runtime>,
        handle: AuthenticatorHandle,
        connection: Option<Box<dyn UsbConnection>>,
    ) -> Self {
        Self {
            state: Rc::new(UsbSecurityKeyState {
                authenticator: AuthenticatorControl::new(runtime, handle),
                connection: RefCell::new(connection),
                connection_state: Cell::new(ConnectionState::Disconnected),
            }),
        }
    }

    pub fn authenticator(&self) -> &AuthenticatorControl {
        &self.state.authenticator
    }

    /// Reattaches this same key, preserving all credentials.
    pub fn connect(&self) -> Result<()> {
        match self.state.connection_state.get() {
            ConnectionState::Connected => return Ok(()),
            ConnectionState::NeedsCleanup => self.disconnect()?,
            ConnectionState::Disconnected => {}
        }
        // An HTTP failure can arrive after the server applied the change.
        self.state
            .connection_state
            .set(ConnectionState::NeedsCleanup);
        let authenticator = self.authenticator();
        let connected = (|| {
            authenticator
                .runtime
                .block_on(authenticator.handle.attach())?;
            if let Some(connection) = self.state.connection.borrow_mut().as_mut() {
                connection.connect()?;
            }
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = connected {
            return match self.disconnect() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!(
                    "also failed to roll back USB connection: {cleanup:#}"
                ))),
            };
        }
        self.state.connection_state.set(ConnectionState::Connected);
        Ok(())
    }

    /// Removes only this key's OS attachment; credentials remain available.
    pub fn disconnect(&self) -> Result<()> {
        if self.state.connection_state.get() == ConnectionState::Disconnected {
            return Ok(());
        }
        self.state
            .connection_state
            .set(ConnectionState::NeedsCleanup);
        if let Some(connection) = self.state.connection.borrow_mut().as_mut() {
            connection.disconnect()?;
        }
        let authenticator = self.authenticator();
        authenticator
            .runtime
            .block_on(authenticator.handle.detach())?;
        self.state
            .connection_state
            .set(ConnectionState::Disconnected);
        Ok(())
    }
}

impl Drop for UsbSecurityKeyState {
    fn drop(&mut self) {
        // Disconnect the OS attachment before the authenticator field deletes the server resource.
        if let Some(connection) = self.connection.get_mut().as_mut()
            && let Err(error) = connection.disconnect()
        {
            eprintln!(
                "Failed to disconnect Security Key {}: {error:#}",
                self.authenticator.id()
            );
        }
    }
}

impl std::fmt::Debug for UsbSecurityKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UsbSecurityKey")
            .field("id", &self.authenticator().id())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticator::ApprovalMode;
    use keyringd_test_usbip::authenticator_server::client::AuthenticatorServerClient;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn with_keys(
        check: impl FnOnce(Vec<UsbSecurityKey>, &Arc<Mutex<Vec<String>>>, &Arc<Mutex<Option<String>>>),
    ) {
        // Only the OS connection is substituted. The actual synchronous owner
        // and HTTP client execute all attachment, consent and deletion calls.
        struct RecordingConnection {
            id: String,
            events: Arc<Mutex<Vec<String>>>,
            fail_next: Arc<Mutex<Option<String>>>,
        }
        impl UsbConnection for RecordingConnection {
            fn connect(&mut self) -> Result<()> {
                self.events
                    .lock()
                    .unwrap()
                    .push(format!("USB connect {}", self.id));
                fail_if_requested(&self.fail_next, &format!("USB connect {}", self.id))
            }
            fn disconnect(&mut self) -> Result<()> {
                self.events
                    .lock()
                    .unwrap()
                    .push(format!("USB disconnect {}", self.id));
                fail_if_requested(&self.fail_next, &format!("USB disconnect {}", self.id))
            }
        }
        struct Server {
            stop: Arc<AtomicBool>,
            thread: Option<std::thread::JoinHandle<()>>,
        }
        impl Drop for Server {
            fn drop(&mut self) {
                self.stop.store(true, Ordering::SeqCst);
                if let Some(thread) = self.thread.take() {
                    thread.join().unwrap();
                }
            }
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let fail_next = Arc::new(Mutex::new(None));
        let server_failure = Arc::clone(&fail_next);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("HTTP fixture: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                let mut headers = String::new();
                loop {
                    headers.clear();
                    assert!(
                        reader.read_line(&mut headers).unwrap() > 0,
                        "HTTP client closed its request headers"
                    );
                    if headers == "\r\n" {
                        break;
                    }
                }
                let mut fields = request.split_whitespace();
                let method = fields.next().unwrap();
                let path = fields.next().unwrap();
                recorded
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push(format!("{method} {path}"));
                if fail_if_requested(&server_failure, &format!("{method} {path}")).is_err() {
                    let body = r#"{"error":"injected failure"}"#;
                    write!(stream, "HTTP/1.0 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                    continue;
                }
                let id = path.split('/').nth(2).unwrap().parse::<u64>().unwrap();
                let body = if path.ends_with("/requests/pending") {
                    serde_json::json!([{
                        "id": id * 100, "kind": "get-assertion", "rpId": "ssh:keyringd",
                        "userPresenceRequired": true, "userVerificationRequired": true,
                        "permissions": ["ga"]
                    }])
                } else {
                    serde_json::json!({"id": id.to_string(), "busId": format!("fixture-{id}"),
                        "connected": true, "consentRequests": id * 10})
                }
                .to_string();
                write!(
                    stream,
                    "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        let _server = Server {
            stop,
            thread: Some(thread),
        };
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        );
        let client = AuthenticatorServerClient::new(address);
        let keys = (1..=3)
            .map(|id| {
                let id = id.to_string();
                let connection = RecordingConnection {
                    id: id.clone(),
                    events: Arc::clone(&events),
                    fail_next: Arc::clone(&fail_next),
                };
                let key = UsbSecurityKey::new(
                    Arc::clone(&runtime),
                    client.authenticator(id).unwrap(),
                    Some(Box::new(connection)),
                );
                key.connect().unwrap();
                key
            })
            .collect();
        check(keys, &events, &fail_next);
    }

    keyringd_testkit::scenario! {
        group "UsbSecurityKey::connect" {
            case "Reconnect uses the same key without recreating or resetting credentials" {
                verify "reattach the original authenticator and preserve its stored credentials" {
                    with_keys(|keys, events, _| {
                        let key = &keys[0];
                        events.lock().unwrap().clear();
                        key.disconnect().unwrap();
                        key.connect().unwrap();
                        assert_eq!(key.authenticator().info().unwrap().id, "1");
                        assert_eq!(
                            *events.lock().unwrap(),
                            [
                                "USB disconnect 1",
                                "POST /authenticators/1/detach",
                                "POST /authenticators/1/attach",
                                "USB connect 1",
                                "GET /authenticators/1",
                            ]
                        );
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbSecurityKey::connect" {
            case "Cloned key is detached and deleted only after the last owner is dropped" {
                verify "retain the attachment until the last cloned handle is released" {
                    with_keys(|mut keys, events, _| {
                        let key = keys.pop().unwrap();
                        let last = key.clone();
                        events.lock().unwrap().clear();
                        drop(key);
                        assert!(events.lock().unwrap().is_empty());
                        assert_eq!(last.authenticator().info().unwrap().id, "3");
                        events.lock().unwrap().clear();
                        drop(last);
                        assert_eq!(
                            *events.lock().unwrap(),
                            ["USB disconnect 3", "DELETE /authenticators/3"]
                        );
                    });
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UsbSecurityKey::connect" {
            case "Three keys keep approval and request operations scoped to their own ids" {
                verify "route consent and requests by authenticator ID with three active keys" {
                    with_keys(|keys, events, _| {
                        events.lock().unwrap().clear();
                        for (index, key) in keys.iter().enumerate() {
                            key.authenticator().set_approval_mode(if index == 1 {
                                ApprovalMode::Automatic
                            } else {
                                ApprovalMode::Manual
                            })
                            .unwrap();
                            assert_eq!(key.authenticator().request_count().unwrap(), (index as u64 + 1) * 10);
                            let pending = key.authenticator().pending_requests().unwrap();
                            assert_eq!(pending.len(), 1);
                            assert_eq!(pending[0].id, (index as u64 + 1) * 100);
                            key.authenticator().approve(pending[0].id).unwrap();
                        }
                        let recorded = events.lock().unwrap();
                        for (id, mode) in [(1, "manual"), (2, "automatic"), (3, "manual")] {
                            assert!(recorded.contains(&format!("POST /authenticators/{id}/mode/{mode}")));
                            assert!(recorded.contains(&format!(
                                "POST /authenticators/{id}/requests/{}/approve",
                                id * 100
                            )));
                        }
                    });
                }
            }
        }
    }
    fn fail_if_requested(next: &Mutex<Option<String>>, operation: &str) -> Result<()> {
        let mut next = next.lock().unwrap();
        if next.as_deref() == Some(operation) {
            next.take();
            anyhow::bail!("injected failure: {operation}");
        }
        Ok(())
    }

    keyringd_testkit::scenario! {
        group "UsbSecurityKey connection recovery" {
            case "Partial attachment and detachment failures can be retried for the same key" {
                verify "failed connect rolls back and failed detach is completed before reconnect" {
                    with_keys(|keys, events, fail| {
                        let key = &keys[0];
                        key.disconnect().unwrap();
                        *fail.lock().unwrap() = Some("USB connect 1".into());
                        let error = key.connect().unwrap_err();
                        assert!(format!("{error:#}").contains("injected failure: USB connect 1"));
                        assert_eq!(events.lock().unwrap().last().unwrap(), "POST /authenticators/1/detach");
                        key.connect().unwrap();
                        *fail.lock().unwrap() = Some("POST /authenticators/1/detach".into());
                        key.disconnect().unwrap_err();
                        events.lock().unwrap().clear();
                        key.connect().unwrap();
                        assert_eq!(*events.lock().unwrap(), ["USB disconnect 1", "POST /authenticators/1/detach", "POST /authenticators/1/attach", "USB connect 1"]);
                        assert_eq!(key.authenticator().info().unwrap().id, "1");
                    });
                }
            }
        }
    }
}
