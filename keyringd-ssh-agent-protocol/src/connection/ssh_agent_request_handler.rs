use crate::{SshAgentRequest, SshAgentResponse};
use std::sync::Arc;

pub trait SshAgentRequestHandler: Sync {
    fn handle(
        &self,
        request: SshAgentRequest,
    ) -> impl std::future::Future<Output = SshAgentResponse> + Send;
}

impl<H> SshAgentRequestHandler for Arc<H>
where
    H: SshAgentRequestHandler + Send + ?Sized,
{
    fn handle(
        &self,
        request: SshAgentRequest,
    ) -> impl std::future::Future<Output = SshAgentResponse> + Send {
        self.as_ref().handle(request)
    }
}
