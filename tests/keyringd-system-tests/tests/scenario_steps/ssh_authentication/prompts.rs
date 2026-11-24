use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::features::pin_prompt_window::PinPromptWindowDriver;
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::application::features::security_key_window::SecurityKeyWindowDriver;
use keyringd_system_tests::environment::SystemTestEnvironment;

pub fn wait_closed(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    environment
        .application()
        .qr_prompt_window()?
        .wait_closed()?;
    environment
        .application()
        .security_key_window()?
        .wait_closed()?;
    #[cfg(unix)]
    environment
        .application()
        .pin_prompt_window()?
        .wait_closed()?;
    #[cfg(windows)]
    environment
        .application()
        .wait_for_fido_operations_to_finish()?;
    Ok(())
}
