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
