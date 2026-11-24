use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::environment::SystemTestEnvironment;
use std::time::{Duration, Instant};

/// Wait for the asynchronous store update, keeping expectations in the case.
pub fn wait_for(
    environment: &mut impl SystemTestEnvironment,
    expected: impl Fn(&[serde_json::Value]) -> bool,
) -> Result<Vec<serde_json::Value>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let profiles = environment.application().stored_profiles()?;
        if expected(&profiles) {
            return Ok(profiles);
        }
        ensure!(
            Instant::now() < deadline,
            "profile store did not converge: {profiles:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}
