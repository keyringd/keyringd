//! Host locks and resources owned by the Linux testbed.
mod run_lock;
pub(crate) mod testbed;
pub(crate) use run_lock::RunLock;
pub use testbed::TestbedExecutionRequest;
pub(crate) use testbed::{AuthenticatorEnvironment, LinuxTestbed};
