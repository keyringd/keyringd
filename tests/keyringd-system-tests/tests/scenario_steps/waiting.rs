use anyhow::{Result, ensure};
use std::time::{Duration, Instant};

pub(crate) fn until(description: &str, mut observe: impl FnMut() -> Result<bool>) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if observe()? {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
