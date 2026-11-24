use anyhow::{Result, ensure};
use keyringd_system_tests::application::features::profiles::ReplaceDeviceKeyWindowDriver;
use keyringd_system_tests::authenticator::UsbSecurityKey;
use std::time::{Duration, Instant};

pub fn approve(window: &impl ReplaceDeviceKeyWindowDriver, key: &UsbSecurityKey) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut approved = 0;
    while !window.result_visible()? {
        ensure!(
            Instant::now() < deadline,
            "Device Key replacement did not finish"
        );
        for request in key.authenticator().pending_requests()? {
            key.authenticator().approve(request.id)?;
            approved += 1;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    ensure!(
        approved > 0,
        "Device Key replacement finished without Security Key consent"
    );
    key.authenticator().wait_for_no_pending_requests()?;
    Ok(())
}
