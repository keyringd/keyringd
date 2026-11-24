use super::ProfileSetupCompletion;
use anyhow::Result;

pub trait CompletionPageDriver {
    fn content(&mut self) -> Result<ProfileSetupCompletion>;
    fn done(&mut self) -> Result<()>;
}
