use anyhow::Result;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::application::features::security_key_window::SecurityKeyWindowDriver;
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::SshSession;
use std::time::{Duration, Instant};

pub fn assert_available(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    let window = environment.application().qr_prompt_window()?;
    let enabled = window.security_key_enabled()?;
    anyhow::ensure!(
        enabled,
        "Security Key transport must be available for USB SSH authentication"
    );
    Ok(())
}

pub fn approve_requests(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    session: &mut impl SshSession,
) -> Result<usize> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut approvals = 0;
    while !session.is_finished()? {
        anyhow::ensure!(
            Instant::now() < deadline,
            "Security Key SSH authentication timed out"
        );
        for request in key.authenticator().pending_requests()? {
            if approvals == 0 {
                let window = environment.application().security_key_window()?;
                window.wait_open()?;
            }
            key.authenticator().approve(request.id)?;
            approvals += 1;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(approvals > 0, "SSH finished without Security Key consent");
    Ok(approvals)
}

/// Enter the PIN when requested; Windows may reuse a cached authorization.
pub fn enter_pin(
    environment: &mut impl SystemTestEnvironment,
    session: &mut impl SshSession,
    pin: &str,
) -> Result<()> {
    #[cfg(unix)]
    {
        use keyringd_system_tests::application::features::pin_prompt_window::PinPromptWindowDriver;
        let window = environment.application().pin_prompt_window()?;
        window.enter_pin(pin)?;
        window.submit()?;
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    #[cfg(windows)]
    let mut entered = false;
    while !session.is_finished()? {
        anyhow::ensure!(
            Instant::now() < deadline,
            "PIN SSH authentication timed out"
        );
        #[cfg(windows)]
        if !entered {
            let window = environment.application().security_key_window()?;
            if window.pin_visible()? {
                window.enter_pin(pin)?;
                entered = true;
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}
