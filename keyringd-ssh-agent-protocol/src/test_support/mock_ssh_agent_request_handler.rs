use keyringd_ssh_agent_protocol::{SshAgentRequest, SshAgentRequestHandler, SshAgentResponse};
use std::future::Future;

mockall::mock! {
    /// HANDLER's mock that sets the response/wait to the SSH Agent's request.
    pub(crate) SshAgentRequestHandler {}
    impl SshAgentRequestHandler for SshAgentRequestHandler {
        fn handle(&self, request: SshAgentRequest) -> impl Future<Output = SshAgentResponse> + Send;
    }
}

pub(crate) fn empty_identities() -> MockSshAgentRequestHandler {
    let mut handler = MockSshAgentRequestHandler::new();
    handler.expect_handle().returning(|request| {
        Box::pin(async move {
            match request {
                SshAgentRequest::Identities => SshAgentResponse::IdentitiesAnswer {
                    identities: Vec::new(),
                },
                _ => SshAgentResponse::Failure,
            }
        })
    });
    handler
}

pub(crate) fn pending_handler() -> MockSshAgentRequestHandler {
    let mut handler = MockSshAgentRequestHandler::new();
    handler
        .expect_handle()
        .returning(|_| Box::pin(std::future::pending()));
    handler
}
