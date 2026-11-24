use anyhow::Result;

/// The OS attachment belonging to exactly one virtual USB Security Key.
pub(crate) trait UsbConnection {
    fn connect(&mut self) -> Result<()>;
    fn disconnect(&mut self) -> Result<()>;
}
