use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
use keyringd_system_tests::application::features::profile_setup::{
    ProfileSetupStep, ProfileSetupWizardDriver,
};
use keyringd_system_tests::authenticator::{ApprovalMode, UsbSecurityKey};
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_test_usbip::authenticator_server::client::ConsentKind;

pub fn create_with_built_in_uv(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) {
    super::form::submit_creation(environment, name).unwrap();
    super::transport_choices::assert_creation(environment).unwrap();
    environment
        .application()
        .profile_setup_window()
        .wizard()
        .unwrap()
        .interaction_page()
        .unwrap()
        .qr_view()
        .unwrap()
        .press_use_security_key()
        .unwrap();
    for (permission, next) in [
        ("mc", ProfileSetupStep::AuthorizeNewDevice),
        ("ga", ProfileSetupStep::Completed),
    ] {
        let request = super::security_key::wait_for_request(environment, key).unwrap();
        super::security_key_prompt::assert_uv_prompt(
            &environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .waiting_view()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(request.kind, ConsentKind::PinUvAuthToken);
        assert!(request.user_verification_required, "{request:?}");
        assert!(request.grants(permission), "{request:?}");

        super::security_key::approve_step_with_presence(environment, key, next);
    }
    key.authenticator()
        .set_approval_mode(ApprovalMode::Automatic)
        .unwrap();
}

pub fn recover_with_built_in_uv(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) {
    super::form::submit_recovery(environment, name).unwrap();
    super::transport_choices::assert_recovery(environment).unwrap();
    environment
        .application()
        .profile_setup_window()
        .wizard()
        .unwrap()
        .interaction_page()
        .unwrap()
        .qr_view()
        .unwrap()
        .press_use_security_key()
        .unwrap();
    for next in [
        ProfileSetupStep::AuthorizeRecoveredDevice,
        ProfileSetupStep::Completed,
    ] {
        let request = super::security_key::wait_for_request(environment, key).unwrap();
        super::security_key_prompt::assert_uv_prompt(
            &environment
                .application()
                .profile_setup_window()
                .wizard()
                .unwrap()
                .interaction_page()
                .unwrap()
                .waiting_view()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(request.kind, ConsentKind::PinUvAuthToken);
        assert!(request.user_verification_required, "{request:?}");
        assert!(request.grants("ga"), "{request:?}");

        super::security_key::approve_step_with_presence(environment, key, next);
    }
    key.authenticator()
        .set_approval_mode(ApprovalMode::Automatic)
        .unwrap();
}

pub fn recover_with_pin(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
) {
    super::form::submit_recovery(environment, name).unwrap();
    super::transport_choices::assert_recovery(environment).unwrap();
    environment
        .application()
        .profile_setup_window()
        .wizard()
        .unwrap()
        .interaction_page()
        .unwrap()
        .qr_view()
        .unwrap()
        .press_use_security_key()
        .unwrap();
    for next in [
        ProfileSetupStep::AuthorizeRecoveredDevice,
        ProfileSetupStep::Completed,
    ] {
        super::security_key::enter_pin(environment, key, "1234", false).unwrap();
        let request = super::security_key::wait_for_request(environment, key).unwrap();
        if request.kind == ConsentKind::PinUvAuthToken {
            assert!(request.grants("ga"), "{request:?}");
        } else {
            assert_eq!(request.kind, ConsentKind::GetAssertion);
        }

        super::security_key::approve_step_with_presence(environment, key, next);
    }
    key.authenticator()
        .set_approval_mode(ApprovalMode::Automatic)
        .unwrap();
}

pub fn create_with_pin(
    environment: &mut impl SystemTestEnvironment,
    key: &UsbSecurityKey,
    name: &str,
    new_pin: bool,
) {
    use keyringd_test_usbip::authenticator_server::client::ConsentKind;
    super::form::submit_creation(environment, name).unwrap();
    super::transport_choices::assert_creation(environment).unwrap();
    environment
        .application()
        .profile_setup_window()
        .wizard()
        .unwrap()
        .interaction_page()
        .unwrap()
        .qr_view()
        .unwrap()
        .press_use_security_key()
        .unwrap();
    for (set_pin, permission, expected, next) in [
        (
            new_pin,
            "mc",
            ConsentKind::MakeCredential,
            ProfileSetupStep::AuthorizeNewDevice,
        ),
        (
            false,
            "ga",
            ConsentKind::GetAssertion,
            ProfileSetupStep::Completed,
        ),
    ] {
        super::security_key::enter_pin(environment, key, "1234", set_pin).unwrap();
        let request = super::security_key::wait_for_request(environment, key).unwrap();
        if request.kind == ConsentKind::PinUvAuthToken {
            assert!(request.grants(permission), "{request:?}");
        } else {
            assert_eq!(request.kind, expected);
        }

        super::security_key::approve_step_with_presence(environment, key, next);
    }
    key.authenticator()
        .set_approval_mode(ApprovalMode::Automatic)
        .unwrap();
}
