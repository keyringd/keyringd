use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupStep;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupInteractionPageDriver, ProfileSetupPinViewDriver, ProfileSetupWaitingViewDriver,
};
#[cfg(windows)]
use keyringd_system_tests::application::features::security_key_window::SecurityKeyWindowDriver;
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_test_usbip::authenticator_server::client::{ConsentKind, PendingConsent};
use std::time::{Duration, Instant};

pub fn capture_prompt(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<()> {
    #[cfg(unix)]
    {
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .waiting_view()?
            .capture_screenshot(name)
    }
    #[cfg(windows)]
    {
        environment
            .application()
            .security_key_window()?
            .capture_screenshot(name)
    }
}

pub fn wait_for_request(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
) -> Result<PendingConsent> {
    let request = key
        .authenticator()
        .wait_for_any_request_approving_selection(&[
            ConsentKind::PinUvAuthToken,
            ConsentKind::MakeCredential,
            ConsentKind::GetAssertion,
        ])?;
    wait_for_prompt(environment)?;
    Ok(request)
}

/// Approve one ceremony, then observe its transition before touching another request.
pub fn approve_step(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    next: ProfileSetupStep,
) -> Result<PendingConsent> {
    let request = wait_for_request(environment, key)?;
    key.authenticator().approve(request.id)?;
    super::progress::wait_for_step(environment, next)?;
    Ok(request)
}

pub fn select(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    other: &UsbSecurityKey,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = (key, other);
        wait_for_selection(environment)
    }
    #[cfg(unix)]
    {
        other
            .authenticator()
            .wait_for_request(ConsentKind::Selection)?;
        let selection = key
            .authenticator()
            .wait_for_request(ConsentKind::Selection)?;
        wait_for_selection(environment)?;
        key.authenticator().approve(selection.id)?;
        other.authenticator().wait_for_no_pending_requests()?;
        Ok(())
    }
}

pub fn assert_operation_finished(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
) -> Result<()> {
    key.authenticator().wait_for_no_pending_requests()?;
    #[cfg(unix)]
    let _ = environment;
    #[cfg(windows)]
    environment
        .application()
        .wait_for_fido_operations_to_finish()?;
    #[cfg(windows)]
    environment
        .application()
        .security_key_window()?
        .wait_closed()?;
    Ok(())
}

#[cfg(unix)]
pub fn wait_for_pin(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    new_pin: bool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        for request in key.authenticator().pending_requests()? {
            if request.kind == ConsentKind::Selection {
                key.authenticator().approve(request.id)?;
            }
        }
        if pin_visible(environment, new_pin)? {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "Security Key PIN prompt did not appear"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

#[cfg(unix)]
pub fn enter_pin(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    pin: &str,
    new_pin: bool,
) -> Result<()> {
    wait_for_pin(environment, key, new_pin)?;
    enter_visible_pin(environment, pin, new_pin)
}

/// Returns whether a PIN was entered; native PIN caches may avoid another prompt.
pub fn approve_step_with_pin(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    pin: &str,
    next: ProfileSetupStep,
) -> Result<bool> {
    #[cfg(windows)]
    let deadline = Instant::now() + Duration::from_secs(30);
    #[cfg(unix)]
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut pin_entered = false;
    let mut approved = false;
    while !super::progress::is_step_visible(environment, next)? {
        ensure!(
            Instant::now() < deadline,
            "PIN-protected USB operation did not reach {next:?}"
        );
        if !approved {
            if !pin_entered && pin_visible(environment, false)? {
                enter_visible_pin(environment, pin, false)?;
                pin_entered = true;
            }
            for request in key.authenticator().pending_requests()? {
                key.authenticator().approve(request.id)?;
                if request.kind != ConsentKind::Selection {
                    approved = true;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(pin_entered)
}

fn wait_for_prompt(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    #[cfg(unix)]
    {
        super::security_key_prompt::wait_for_key_prompt(
            &environment
                .application()
                .profile_setup_window()
                .wizard()?
                .interaction_page()?
                .waiting_view()?,
        )
    }
    #[cfg(windows)]
    {
        environment.application().security_key_window()?.wait_open()
    }
}
pub fn wait_for_selection(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    #[cfg(unix)]
    {
        let mut window = environment.application().profile_setup_window();
        let mut wizard = window.wizard()?;
        let mut interaction = wizard.interaction_page()?;
        let view = interaction.waiting_view()?;
        super::security_key_prompt::wait_for_key_selection(&view)
    }
    #[cfg(windows)]
    {
        let window = environment.application().security_key_window()?;
        window.wait_open()
    }
}

fn pin_visible(environment: &mut impl SystemTestEnvironment, new_pin: bool) -> Result<bool> {
    #[cfg(unix)]
    {
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .interaction_page()?
            .pin_view()?
            .visible(new_pin)
    }
    #[cfg(windows)]
    {
        let _ = new_pin;
        environment
            .application()
            .security_key_window()?
            .pin_visible()
    }
}
fn enter_visible_pin(
    environment: &mut impl SystemTestEnvironment,
    pin: &str,
    new_pin: bool,
) -> Result<()> {
    #[cfg(unix)]
    {
        let mut window = environment.application().profile_setup_window();
        let mut wizard = window.wizard()?;
        let mut interaction = wizard.interaction_page()?;
        let mut view = interaction.pin_view()?;
        view.enter(pin, new_pin)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = new_pin;
        let window = environment.application().security_key_window()?;
        window.enter_pin(pin)?;
        Ok(())
    }
}

pub fn wait_for_pin_closed(environment: &mut impl SystemTestEnvironment) -> Result<()> {
    #[cfg(unix)]
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let progress = environment
                .application()
                .profile_setup_window()
                .wizard()?
                .observe()?;
            if !progress.pin_visible {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "security key PIN field did not disappear: {progress:?}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    #[cfg(windows)]
    {
        environment
            .application()
            .security_key_window()?
            .wait_closed()
    }
}

pub fn approve_step_with_presence(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    next: ProfileSetupStep,
) -> PendingConsent {
    let request = approve_step(environment, key, next).unwrap();
    assert!(request.user_presence_required, "{request:?}");
    request
}
