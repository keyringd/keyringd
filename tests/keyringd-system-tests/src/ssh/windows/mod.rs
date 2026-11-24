//! Real SSH fixtures, isolated from UI selectors and scenario policy.
mod agent;
mod current_user;
pub(crate) use current_user::CurrentUser;
mod file_security_descriptor;
mod prepare_sshd;
mod process_job;
mod ssh_server;

pub use agent::{agent_identities, agent_identity};
pub use ssh_server::SshServer;

mod windows_ssh_session;
pub use windows_ssh_session::WindowsSshSession;

mod windows_ssh_servers;
pub use windows_ssh_servers::WindowsSshServers;
