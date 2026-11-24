use anyhow::{Context, Result, ensure};
use keyringd_system_tests::application::features::navigation::windows::UiApplication;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileFormPageDriver, ProfileSetupInteractionPageDriver,
    ProfileSetupProgress, ProfileSetupQrViewDriver, ProfileSetupStep, ProfileSetupWizardDriver,
};
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
#[cfg(windows)]
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::application::platform::windows::UiDriver;
use keyringd_system_tests::environment::windows::TestApp;

pub use keyringd_core::model::ProfileSshIdentityKind as IdentityKind;
use keyringd_system_tests::authenticator::{Authenticators, HybridPhone};
use keyringd_testkit::PxpAdvertisementRelay;

pub fn prepare_ssh(
    name: &str,
    kind: IdentityKind,
) -> Result<(TestApp, UiDriver, crate::ssh_profile::SshProfile)> {
    let app = TestApp::launch(&Default::default())?;
    let driver = UiDriver::new()?;
    let ui = UiApplication::new(&driver, app.pid());
    let profiles = ui
        .profiles_window()
        .context("open installed companion profile window before USB setup")?;
    let mut wizard = profiles.start_profile_creation()?;
    wizard.profile_form_page()?.enter_profile_name(name)?;
    wizard.profile_form_page()?.select_new_passkey()?;
    wizard.profile_form_page()?.submit()?;
    wizard
        .interaction_page()?
        .qr_view()?
        .press_use_security_key()?;
    wizard.completion_page()?.done()?;
    assert_profile_visible(&profiles, name)?;
    let profile = read_profile(&app, &driver, name, kind)?;
    Ok((app, driver, profile))
}

pub fn prepare_hybrid_ssh(
    name: &str,
    kind: IdentityKind,
) -> Result<(
    TestApp,
    UiDriver,
    crate::ssh_profile::SshProfile,
    HybridPhone,
)> {
    let runtime = std::sync::Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?,
    );
    let authenticators = Authenticators::from_environment(runtime)?;
    let phone = authenticators.create_hybrid_phone(Default::default())?;
    let app = TestApp::launch(
        &keyringd_system_tests::application::ApplicationConfiguration {
            accept_remote_hybrid_advertisements: true,
            ..Default::default()
        },
    )?;
    let driver = UiDriver::new()?;
    let ui = UiApplication::new(&driver, app.pid());
    let profiles = ui
        .profiles_window()
        .context("open installed companion profile window before Hybrid setup")?;
    let mut wizard = profiles.start_profile_creation()?;
    wizard.profile_form_page()?.enter_profile_name(name)?;
    wizard.profile_form_page()?.select_new_passkey()?;
    wizard.profile_form_page()?.submit()?;
    let relay = PxpAdvertisementRelay::new(app.agent_pipe()?);
    for step in 0..2 {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let progress = wizard.observe()?;
            let checked = if step == 0 {
                assert_choices(
                    &progress, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::CreatePasskey,
                )
            } else {
                assert_step(
                    &progress, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeNewDevice,
                )
            };
            match checked {
                Ok(()) => break,
                Err(error) => {
                    ensure!(std::time::Instant::now() < deadline, "{error:#}");
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
        }
        let (uri, _) = wizard.interaction_page()?.qr_view()?.read_hybrid_qr(None)?;
        relay.send(&phone.scan_hybrid_qr(&uri)?.service_data()?)?;
    }
    wizard.completion_page()?.done()?;
    assert_profile_visible(&profiles, name)?;
    let profile = read_profile(&app, &driver, name, kind)?;
    Ok((app, driver, profile, phone))
}

pub fn approve_hybrid_signing(
    app: &TestApp,
    ui: &UiApplication<'_>,
    phone: &HybridPhone,
) -> Result<()> {
    let (uri, _) = ui.qr_prompt_window().read_hybrid_qr()?;
    PxpAdvertisementRelay::new(app.agent_pipe()?)
        .send(&phone.scan_hybrid_qr(&uri)?.service_data()?)?;
    Ok(())
}

fn read_profile(
    app: &TestApp,
    driver: &UiDriver,
    name: &str,
    kind: IdentityKind,
) -> Result<crate::ssh_profile::SshProfile> {
    let ui = UiApplication::new(driver, app.pid());
    let label = match kind {
        IdentityKind::DeviceCertificate => "Certificate Authority Public Key",
        IdentityKind::DeviceKey => "Device Key Public Key",
        IdentityKind::Passkey => "Passkey Public Key",
    };
    let keys = ui
        .profiles_window()
        .context("reopen installed companion profile window after setup")?
        .open_trust_entries(name)?;
    let trust_entry = keys.copy_entry(label)?;
    keys.close()?;
    let identity = keyringd_system_tests::ssh::windows::agent_identity(app, kind)?;
    Ok(crate::ssh_profile::SshProfile {
        trust_entry,
        identity,
    })
}

fn assert_step(progress: &ProfileSetupProgress, step: ProfileSetupStep) -> Result<()> {
    ensure!(
        progress.heading == step.title() || progress.texts.iter().any(|text| text == step.title()),
        "expected setup step {step:?}: {progress:?}"
    );
    Ok(())
}

fn assert_choices(progress: &ProfileSetupProgress, step: ProfileSetupStep) -> Result<()> {
    assert_step(progress, step)?;
    let instruction = match step {
        ProfileSetupStep::CreatePasskey => {
            "Scan with the phone or tablet where you want to create the Passkey."
        }
        ProfileSetupStep::FindPasskey => {
            "Scan with the phone or tablet that contains your existing passkey."
        }
        _ => anyhow::bail!("{step:?} is not an authenticator choice page"),
    };
    for expected in [
        instruction,
        "Bluetooth must be enabled on both devices.",
        "Use a security key instead?",
        "You can also use a connected USB security key.",
    ] {
        ensure!(
            progress.texts.iter().any(|text| text == expected),
            "missing setup instruction {expected:?}: {progress:?}"
        );
    }
    ensure!(
        progress.security_key_enabled == Some(true),
        "Security Key choice is unavailable: {progress:?}"
    );
    Ok(())
}

fn assert_profile_visible(window: &impl ProfilesWindowDriver, name: &str) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        if window.profile_visible(name)? {
            return Ok(());
        }
        ensure!(
            std::time::Instant::now() < deadline,
            "timed out waiting for profile {name:?} to be visible"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}
