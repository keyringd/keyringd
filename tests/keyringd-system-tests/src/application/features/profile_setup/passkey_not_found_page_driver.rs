use anyhow::Result;

pub trait PasskeyNotFoundPageDriver {
    fn continue_with_new_passkey(&mut self) -> Result<()>;
    fn try_another_device(&mut self) -> Result<()>;
}
