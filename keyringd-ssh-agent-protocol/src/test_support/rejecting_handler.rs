use super::MockSshAgentRequestHandler;
use crate::SshAgentResponse;

pub(crate) fn rejecting_handler() -> MockSshAgentRequestHandler {
    let mut handler = MockSshAgentRequestHandler::new();
    handler
        .expect_handle()
        .returning(|_| Box::pin(async { SshAgentResponse::Failure }));
    handler
}
