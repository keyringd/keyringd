use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use keyringd_system_tests::application::features::about::{
    AboutApplicationInfo, AboutWindowDriver,
};

pub fn assert_content(about: &impl AboutWindowDriver) -> Result<()> {
    // About can become accessible before all native labels are populated.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let info = about.application_info()?;
        let result = validate(&info);
        match result {
            Ok(()) => return Ok(()),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn validate(info: &AboutApplicationInfo) -> Result<()> {
    let version = keyringd_testkit::product_version::VERSION;
    for text in ["Keyringd", "Keyringd maintainers"] {
        ensure!(
            info.texts.iter().any(|actual| actual == text),
            "missing About text {text:?}: {info:?}"
        );
    }
    ensure!(
        info.texts
            .iter()
            .any(|actual| actual == &format!("Version {version}")),
        "missing About version {version:?}: {info:?}"
    );
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        ensure!(
            info.texts
                .iter()
                .any(|actual| actual == "License: Apache License 2.0"),
            "missing About license: {info:?}"
        );
        #[cfg(target_os = "macos")]
        for action in [
            "about.website",
            "about.issues",
            "about.license",
            "about.third-party",
        ] {
            ensure!(
                info.enabled_actions.iter().any(|actual| actual == action),
                "About action {action:?} is not visible and enabled: {info:?}"
            );
        }
    }
    #[cfg(target_os = "windows")]
    {
        ensure!(
            info.texts
                .iter()
                .any(|actual| actual.contains("Apache License 2.0")),
            "missing About license: {info:?}"
        );
        for action in ["Website", "Report an issue"] {
            ensure!(
                info.enabled_actions.iter().any(|actual| actual == action),
                "About action {action:?} is not visible and enabled: {info:?}"
            );
        }
    }
    Ok(())
}
