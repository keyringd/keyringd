use keyringd_ssh_agent_protocol::SshAgentListener;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

use super::user_only_security_descriptor::UserOnlySecurityDescriptor;

pub(crate) struct WindowsNamedPipeListener {
    pipe_name: String,
    first_instance: bool,
}

impl WindowsNamedPipeListener {
    pub(crate) fn new(pipe_name: String) -> Self {
        Self {
            pipe_name,
            first_instance: true,
        }
    }
}

impl SshAgentListener for WindowsNamedPipeListener {
    type Stream = NamedPipeServer;

    async fn accept(&mut self) -> std::io::Result<Self::Stream> {
        let server = {
            let descriptor = UserOnlySecurityDescriptor::new()?;
            let mut attributes = descriptor.attributes();
            // The descriptor and attributes remain valid until CreateNamedPipe
            // copies them; neither is needed while waiting for a connection.
            unsafe {
                ServerOptions::new()
                    .first_pipe_instance(self.first_instance)
                    .reject_remote_clients(true)
                    .create_with_security_attributes_raw(
                        &self.pipe_name,
                        (&mut attributes as *mut windows::Win32::Security::SECURITY_ATTRIBUTES)
                            .cast(),
                    )?
            }
        };
        self.first_instance = false;
        server.connect().await?;
        Ok(server)
    }
}
