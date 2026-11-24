use crate::application::SystemTestApplication;
use crate::authenticator::Authenticators;
use crate::ssh::SshServers;
use anyhow::Result;

pub trait SystemTestEnvironment: Sized {
    type Application: SystemTestApplication;
    type SshServers: SshServers;
    #[cfg(target_os = "linux")]
    type ApplicationSocketActivation: super::SocketActivationDriver;
    #[cfg(target_os = "linux")]
    fn prepare_application_socket_activation(
        &mut self,
    ) -> Result<Self::ApplicationSocketActivation>;
    fn new() -> Result<Self>;
    /// Whether this environment can reproduce disabled Bluetooth for a test.
    fn can_disable_bluetooth() -> bool;
    fn application(&mut self) -> &mut Self::Application;
    fn authenticators(&self) -> &Authenticators;
    fn ssh_servers(&self) -> &Self::SshServers;
}
