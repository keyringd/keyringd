use crate::scenario_steps;
use keyringd_system_tests::application::ApplicationConfiguration;
use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupInteractionPageDriver, ProfileSetupQrViewDriver,
};
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::ProfilesWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::application::features::profiles::TrustEntriesWindowDriver;
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_system_tests::authenticator::ApprovalMode;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
use keyringd_system_tests::ssh::SshSession;
use keyringd_system_tests::ssh::{SshServers, SshTestServer};
use keyringd_testkit::{evidence_log, evidence_text, scenario};

scenario! {
    group "SSH Authentication / Direct Passkey" {
        case "Authenticates with a registered Passkey" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires AuthenticatorServer and the OS Security Key environment";
            given "Create a profile from Security Key with built-in UV" {
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
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "ssh-passkey",
                )
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
                scenario_steps::profile_setup::completion::wait_for(&mut environment).unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .capture_screenshot("Profile creation completed")
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
                let passkey = scenario_steps::profile_management::public_keys::copy_passkey(
                    &mut environment,
                    "ssh-passkey",
                )
                .unwrap();
                {
                    let mut profiles = environment.application().profiles_window().unwrap();
                    let mut public_keys = profiles.open_trust_entries("ssh-passkey").unwrap();
                    public_keys
                        .capture_screenshot("Direct SSH Public Keys")
                        .unwrap();
                    public_keys.close().unwrap();
                }
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.comment == "keyringd ssh-passkey passkey")
                    .unwrap();
            }
            then "Only Passkey's public key is copied as SK key for direct authentication" {
                assert!(
                    passkey
                        .starts_with("sk-ecdsa-sha2-nistp256@openssh.com "),
                    "{}",
                    passkey
                );
                assert!(
                    passkey.ends_with(" keyringd ssh-passkey passkey"),
                    "{}",
                    passkey
                );
                assert_eq!(identity.algorithm, "sk-ecdsa-sha2-nistp256@openssh.com");
                evidence_text!("Passkey Public Key", &passkey);
            }
            when "Connect to the real SSH server and select Security Key from the Hybrid selection screen to approve" {
                key.authenticator().set_approval_mode(ApprovalMode::Manual).unwrap();
                assert!(
                    environment.application().uses_native_fido().unwrap(),
                    "Passkey SSH must use native WebAuthn"
                );
                let ssh_server = environment.ssh_servers().start(&passkey).unwrap();
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
                    .capture_screenshot("SSH transport choices")
                    .unwrap();
                environment
                    .application()
                    .qr_prompt_window()
                    .unwrap()
                    .press_use_security_key()
                    .unwrap();
                let approvals = scenario_steps::ssh_authentication::security_key::approve_requests(
                    &mut environment,
                    &key,
                    &mut ssh,
                )
                .unwrap();
                assert!(approvals > 0);
                let connection = ssh.finish().unwrap();
                scenario_steps::ssh_authentication::prompts::wait_closed(&mut environment).unwrap();
                key.authenticator().wait_for_no_pending_requests().unwrap();
                environment
                    .application()
                    .capture_screenshot("Profiles after SSH authentication")
                    .unwrap();
            }
            then "Selected Passkey is accepted and the command is successful instead of the certificate" {
                connection.assert_authenticated(&identity);
                evidence_log!("Passkey SSH diagnostics", &connection.stderr);
                evidence_log!("sshd authentication log", &connection.server_log);
            }
        }
    }
}
