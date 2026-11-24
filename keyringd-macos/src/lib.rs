// Native FIDO futures need a deeper Send check than the compiler's default limit.
#![recursion_limit = "256"]

mod app;
mod system;

#[cfg(test)]
mod tests;

/// Runs the UI test fixture: the product UI with test backends and the
/// in-process accessibility driver, driven over a case-private control channel.
#[cfg(feature = "test-support")]
pub fn run_ui_test_app() -> anyhow::Result<()> {
    app::ui_test_support::run()
}

/// Runs the product.
///
/// A startup failure has to reach someone who launched the application from
/// Finder, where stderr goes nowhere, so it is reported in a dialog as well.
pub fn run() -> anyhow::Result<()> {
    let result = (|| {
        system::MacOsLogging::init()?;
        app::MacOsEntryPoint::run()
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            keyringd_core::util::FailureDiagnostics::record_error("macOS entry point", &error);
            if let Some(mtm) = objc2::MainThreadMarker::new() {
                use objc2_app_kit::{
                    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
                };
                use objc2_foundation::NSString;
                let application = NSApplication::sharedApplication(mtm);
                application.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                #[allow(deprecated)]
                application.activateIgnoringOtherApps(true);
                let alert = NSAlert::new(mtm);
                alert.setAlertStyle(NSAlertStyle::Critical);
                alert.setMessageText(&NSString::from_str("Keyringd could not start"));
                alert.setInformativeText(&NSString::from_str(&format!("Details:\n{error:#}")));
                alert.addButtonWithTitle(&NSString::from_str("Close"));
                alert.runModal();
            }
            Err(error)
        }
    }
}
