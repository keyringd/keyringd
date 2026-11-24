#[cfg(unix)]
use crate::scenario_steps;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
#[cfg(unix)]
use keyringd_system_tests::authenticator::ApprovalMode;

#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupStep;
#[cfg(unix)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(unix)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
#[cfg(unix)]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(unix)]
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_test_usbip::authenticator_server::client::{AuthenticatorProfile, ConsentKind};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyringd_testkit::evidence_text;
use keyringd_testkit::scenario;

scenario! {
    group "Profile Setup / USB Keys with Mixed UV and PIN Capabilities" {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        case "Uses built-in UV when a USB key with built-in UV is selected" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "Prepare PIN keys and internal UV keys manually" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let other_key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                other_key.authenticator()
                    .reset(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .unwrap();
                other_key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Start Passkey creation and select internal UV keys" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "mixed-uv-key",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                    .unwrap();
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
                scenario_steps::profile_setup::security_key::wait_for_selection(&mut environment)
                    .unwrap();
                environment
                    .application()
                    .capture_screenshot("Choose the built-in UV key")
                    .unwrap();
                scenario_steps::profile_setup::security_key::select(
                    &mut environment,
                    &key,
                    &other_key,
                )
                .unwrap();
                let create = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
            }
            then "Selected key to request creation with built-in UV" {
                assert!(create.user_verification_required);
                assert!(create.grants("mc"));
                evidence_text!("Mixed-key MakeCredential token", format!("{create:?}"));
                environment
                    .application()
                    .capture_screenshot("Verify on the selected built-in UV key")
                    .unwrap();
            }
            when "Approve Passkey creation with built-in UVs without entering a PIN" {
                scenario_steps::profile_setup::security_key::approve_step_with_presence(
                    &mut environment,
                    &key,
                    ProfileSetupStep::AuthorizeNewDevice,
                );
                scenario_steps::profile_setup::progress::wait_for_step(&mut environment, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeNewDevice).unwrap();
                let authorize = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
            }
            then "The same UV key requests a Device Key authorized token" {
                assert_eq!(authorize.kind, ConsentKind::PinUvAuthToken);
                assert!(authorize.grants("ga"));
                assert!(authorize.user_verification_required);
                environment
                    .application()
                    .capture_screenshot("Built-in UV authorizes this PC")
                    .unwrap();
            }
            when "Approve Device Key authorization with the same key's internal UV" {
                scenario_steps::profile_setup::security_key::approve_step_with_presence(
                    &mut environment,
                    &key,
                    ProfileSetupStep::Completed,
                );
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profile completed through mixed-key UV")
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
            }
            then "Keys that are not selected are saved without entering a PIN and are unused" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "mixed-uv-key",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "mixed-uv-key");
            }
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]

        case "Uses PIN and touch when a PIN-protected USB key is selected" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and direct CTAP Security Key support";
            given "Prepare the internal UV keys and PIN keys manually" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let other_key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                let configuration = ApplicationConfiguration::default();
                environment.application().start(configuration).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                other_key.authenticator()
                    .reset(AuthenticatorProfile::UvConfiguredPinUnconfigured)
                    .unwrap();
                other_key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                key.authenticator().reset(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
            }
            when "Start creating a Passkey and select a PIN key by touching it" {
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "mixed-pin-key",
                )
                .unwrap();
                scenario_steps::profile_setup::transport_choices::assert_creation(&mut environment)
                    .unwrap();
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
                scenario_steps::profile_setup::security_key::wait_for_selection(&mut environment)
                    .unwrap();
                environment
                    .application()
                    .capture_screenshot("Choose the PIN-protected key")
                    .unwrap();
                scenario_steps::profile_setup::security_key::select(
                    &mut environment,
                    &key,
                    &other_key,
                )
                .unwrap();
            }
            then "Only the selected PIN key requests input and the request for another key is cancelled" {
                scenario_steps::profile_setup::security_key::wait_for_pin(
                    &mut environment,
                    &key,
                    false,
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Only the selected key requests a PIN")
                    .unwrap();
                scenario_steps::profile_setup::security_key::enter_pin(
                    &mut environment,
                    &key,
                    "1234",
                    false,
                )
                .unwrap();
                let create = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
                assert!(create.user_presence_required);
                if create.kind == ConsentKind::PinUvAuthToken {
                    assert!(create.grants("mc"));
                } else {
                    assert_eq!(create.kind, ConsentKind::MakeCredential);
                }
                scenario_steps::profile_setup::security_key_prompt::assert_creation_approval(
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
                evidence_text!("Mixed-key Passkey creation consent", format!("{create:?}"));
            }
            when "Touch Approve a PIN-approved Passkey Creation" {
                scenario_steps::profile_setup::security_key::approve_step_with_presence(
                    &mut environment,
                    &key,
                    ProfileSetupStep::AuthorizeNewDevice,
                );
                scenario_steps::profile_setup::progress::wait_for_step(&mut environment, keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeNewDevice).unwrap();
            }
            then "Re-enter the Device Key authorization PIN with the same PIN key and request a touch" {
                scenario_steps::profile_setup::security_key::wait_for_pin(
                    &mut environment,
                    &key,
                    false,
                )
                .unwrap();
                environment
                    .application()
                    .capture_screenshot("Enter the PIN again for Device Key authorization")
                    .unwrap();
                scenario_steps::profile_setup::security_key::enter_pin(
                    &mut environment,
                    &key,
                    "1234",
                    false,
                )
                .unwrap();
                let authorize = scenario_steps::profile_setup::security_key::wait_for_request(
                    &mut environment,
                    &key,
                )
                .unwrap();
                assert!(authorize.user_presence_required);
                if authorize.kind == ConsentKind::PinUvAuthToken {
                    assert!(authorize.grants("ga"));
                } else {
                    assert_eq!(authorize.kind, ConsentKind::GetAssertion);
                }
                environment
                    .application()
                    .capture_screenshot("Selected PIN key authorizes this PC")
                    .unwrap();
            }
            when "Touch Approve Device Key authorization with the same key" {
                scenario_steps::profile_setup::security_key::approve_step_with_presence(
                    &mut environment,
                    &key,
                    ProfileSetupStep::Completed,
                );
                key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .capture_screenshot("Profile completed through mixed-key PIN")
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
            }
            then "The profile is saved without the internal UV approval and the other key is unused" {
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "mixed-pin-key",
                )
                .unwrap();
                let stored = environment.application().stored_profiles().unwrap();
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0]["displayName"], "mixed-pin-key");
            }
        }
    }
}
