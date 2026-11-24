use anyhow::{Result, ensure};
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, PasskeySelectionPageDriver, ProfileSetupInteractionPageDriver,
    ProfileSetupQrViewDriver, ProfileSetupStep, ProfileSetupWindowDriver, ProfileSetupWizardDriver,
};
use keyringd_system_tests::authenticator::HybridPhone;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_test_usbip::authenticator_server::client::HybridAdvertisement;

pub fn create(
    environment: &mut impl SystemTestEnvironment,
    phone: &HybridPhone,
    name: &str,
) -> Result<[HybridAdvertisement; 2]> {
    super::form::submit_creation(environment, name)?;
    super::transport_choices::assert_creation(environment)?;
    let (first_qr, first_advertisement) = approve_current_operation(environment, phone, None)?;
    super::progress::wait_for_step(environment, ProfileSetupStep::AuthorizeNewDevice)?;
    let (_, second_advertisement) = approve_current_operation(environment, phone, Some(&first_qr))?;
    finish_profile(environment, name)?;
    Ok([first_advertisement, second_advertisement])
}

pub fn recover(
    environment: &mut impl SystemTestEnvironment,
    phone: &HybridPhone,
    name: &str,
    attestation_available: bool,
) -> Result<()> {
    super::form::submit_recovery(environment, name)?;
    super::transport_choices::assert_recovery(environment)?;
    let (first_qr, _) = approve_current_operation(environment, phone, None)?;
    let previous_qr = if attestation_available {
        first_qr.clone()
    } else {
        super::progress::wait_for_step(environment, ProfileSetupStep::RecoverPasskeyPublicKey)?;
        approve_current_operation(environment, phone, Some(&first_qr))?.0
    };
    super::progress::wait_for_step(environment, ProfileSetupStep::AuthorizeRecoveredDevice)?;
    let (certificate_qr, _) = approve_current_operation(environment, phone, Some(&previous_qr))?;
    ensure!(
        first_qr != certificate_qr,
        "recovery reused the discovery Hybrid session"
    );
    finish_profile(environment, name)
}

pub fn discover_credentials(
    environment: &mut impl SystemTestEnvironment,
    phone: &HybridPhone,
    name: &str,
) -> Result<Vec<String>> {
    super::form::submit_recovery(environment, name)?;
    super::transport_choices::assert_recovery(environment)?;
    approve_current_operation(environment, phone, None)?;
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .passkey_selection_page()?
        .candidate_labels()
}

pub fn approve_selected(
    environment: &mut impl SystemTestEnvironment,
    phone: &HybridPhone,
) -> Result<()> {
    super::progress::wait_for_step(environment, ProfileSetupStep::AuthorizeRecoveredDevice)?;
    approve_current_operation(environment, phone, None)?;
    Ok(())
}

fn approve_current_operation(
    environment: &mut impl SystemTestEnvironment,
    phone: &HybridPhone,
    previous_qr: Option<&str>,
) -> Result<(String, HybridAdvertisement)> {
    let (qr, _) = environment
        .application()
        .profile_setup_window()
        .wizard()?
        .interaction_page()?
        .qr_view()?
        .read_hybrid_qr(previous_qr)?;
    ensure!(
        previous_qr != Some(qr.as_str()),
        "each FIDO operation must display a fresh Hybrid session"
    );
    let advertisement = phone.scan_hybrid_qr(&qr)?;
    environment
        .application()
        .send_hybrid_advertisement(&advertisement)?;
    Ok((qr, advertisement))
}

fn finish_profile(environment: &mut impl SystemTestEnvironment, name: &str) -> Result<()> {
    environment
        .application()
        .profile_setup_window()
        .wizard()?
        .completion_page()?
        .done()?;
    crate::scenario_steps::profile_management::profile_list::assert_visible(
        &environment.application().profiles_window()?,
        name,
    )
}
