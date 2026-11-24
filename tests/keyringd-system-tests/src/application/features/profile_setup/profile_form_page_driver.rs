use super::ProfileSetupForm;
use anyhow::Result;

pub trait ProfileFormPageDriver {
    fn content(&mut self) -> Result<ProfileSetupForm>;
    fn enter_profile_name(&self, name: &str) -> Result<()>;
    fn show_advanced_settings(&self) -> Result<()>;
    fn select_new_passkey(&self) -> Result<()>;
    fn select_existing_passkey(&self) -> Result<()>;
    fn submit(&mut self) -> Result<()>;
}
