use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileFormPageDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::environment::SystemTestEnvironment;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(20);

pub fn assert_content(
    environment: &mut impl SystemTestEnvironment,
    name: &str,
    existing: bool,
) -> Result<()> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let form = environment
            .application()
            .profile_setup_window()
            .wizard()?
            .profile_form_page()?
            .content()?;
        if form.name == name
            && form.existing_passkey == existing
            && form.new_passkey != existing
            && form.create_enabled
        {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "profile form did not preserve name={name:?}, existing={existing}: {form:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn assert_create_enabled(
    environment: &mut impl SystemTestEnvironment,
    enabled: bool,
) -> Result<()> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let form = environment
            .application()
            .profile_setup_window()
            .wizard()?
            .profile_form_page()?
            .content()?;
        if form.create_enabled == enabled {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Create did not reach enabled={enabled}: {form:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn assert_message(environment: &mut impl SystemTestEnvironment, message: &str) -> Result<()> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let form = environment
            .application()
            .profile_setup_window()
            .wizard()?
            .profile_form_page()?
            .content()?;
        if form.messages.iter().any(|text| text == message) && form.create_enabled {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "form did not display {message:?} with Create available: {form:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn assert_error(environment: &mut impl SystemTestEnvironment, message: &str) -> Result<()> {
    assert_message(environment, message)?;
    ensure!(
        environment
            .application()
            .profile_setup_window()
            .wizard()?
            .profile_form_page()?
            .content()?
            .create_label
            == "Try again",
        "the failed setup must offer Try again"
    );
    Ok(())
}

pub fn submit_creation(
    environment: &mut impl SystemTestEnvironment,
    profile_name: &str,
) -> Result<()> {
    let mut window = environment.application().profile_setup_window();
    let mut setup = window.wizard()?;
    let mut page = setup.profile_form_page()?;
    page.enter_profile_name(profile_name)?;
    page.select_new_passkey()?;
    page.submit()
}

pub fn submit_recovery(
    environment: &mut impl SystemTestEnvironment,
    profile_name: &str,
) -> Result<()> {
    let mut window = environment.application().profile_setup_window();
    let mut setup = window.wizard()?;
    let mut page = setup.profile_form_page()?;
    page.enter_profile_name(profile_name)?;
    page.select_existing_passkey()?;
    page.submit()
}
