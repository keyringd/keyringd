use anyhow::{Context, Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::security_key_window::SecurityKeyWindowDriver;
use keyringd_system_tests::application::features::ssh_connection_test::ConnectionTestResultWindowDriver;
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_test_usbip::authenticator_server::client::ConsentKind;
use std::time::{Duration, Instant};

/// Approve the selected test key while observing the connection UI independently.
pub fn approve(environment: &mut impl SystemTestEnvironment, key: &UsbSecurityKey) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !environment
        .application()
        .connection_test_result_window()
        .visible()
        .context("observe SSH connection result")?
    {
        ensure!(
            Instant::now() < deadline,
            "connection result did not appear"
        );
        for request in key.authenticator().pending_requests()? {
            if request.kind != ConsentKind::Selection {
                let window = environment.application().security_key_window()?;
                window
                    .wait_open()
                    .context("wait for SSH Security Key approval prompt")?;
            }
            key.authenticator().approve(request.id)?;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    key.authenticator().wait_for_no_pending_requests()?;
    environment
        .application()
        .security_key_window()?
        .wait_closed()?;
    Ok(())
}
