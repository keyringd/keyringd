use crate::scenario_steps;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_test_usbip::authenticator_server::client::AuthenticatorProfile;
use keyringd_testkit::scenario;

scenario! {
    group "SSH Authentication / Security Key PIN" {
        case "Authenticates over SSH with a PIN whether Profiles is open or closed" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Create a Profile with PIN-protected Security Key" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
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
                key.authenticator().reset(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .unwrap();
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                scenario_steps::profile_setup::form::submit_creation(&mut environment, "ssh-pin")
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
                for next in [
                    keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::AuthorizeNewDevice,
                    keyringd_system_tests::application::features::profile_setup::ProfileSetupStep::Completed,
                ] {
                    scenario_steps::profile_setup::security_key::approve_step_with_pin(
                        &mut environment, &key, "1234", next,
                    ).unwrap();
                }
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                let passkey = scenario_steps::profile_management::public_keys::copy_passkey(
                    &mut environment,
                    "ssh-pin",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.comment == "keyringd ssh-pin passkey")
                    .unwrap();
            }
            when "Authenticate with PIN SSH while viewing and closing the profile list" {
                let mut connections = Vec::new();
                for presentation in ["profiles-open", "profiles-closed"] {
                    if presentation == "profiles-closed" {
                        environment
                            .application()
                            .profiles_window()
                            .unwrap()
                            .close()
                            .unwrap();
                    }
                    let ssh_server = environment.ssh_servers().start(&passkey).unwrap();
                    key.authenticator().set_approval_mode(ApprovalMode::Automatic).unwrap();
                    let mut ssh = ssh_server
                        .start_client(
                            &environment.application().agent_endpoint().unwrap(),
                            &identity,
                            "echo keyringd-ssh-ok",
                        )
                        .unwrap();
                    scenario_steps::ssh_authentication::security_key::assert_available(
                        &mut environment,
                    )
                    .unwrap();
                    environment
                        .application()
                        .qr_prompt_window()
                        .unwrap()
                        .capture_screenshot(&format!("SSH transport choices ({presentation})"))
                        .unwrap();
                    environment
                        .application()
                        .qr_prompt_window()
                        .unwrap()
                        .press_use_security_key()
                        .unwrap();
                    scenario_steps::ssh_authentication::security_key::enter_pin(
                        &mut environment,
                        &mut ssh,
                        "1234",
                    )
                    .unwrap();
                    let connection = ssh.finish().unwrap();
                    scenario_steps::ssh_authentication::prompts::wait_closed(&mut environment)
                        .unwrap();
                    key.authenticator().wait_for_no_pending_requests().unwrap();

                    case_execution.attach_log(
                        format!("{presentation} SSH authentication"),
                        &connection.stderr,
                    );
                    case_execution.attach_log(
                        format!("{presentation} sshd authentication"),
                        &connection.server_log,
                    );
                    connections.push(connection);
                }
            }
            then "Succeeded in SSH authentication with the same Passkey in either state" {
                for connection in connections {
                    connection.assert_authenticated(&identity);
                }
            }
        }
    }
}
