#[cfg(windows)]
use crate::scenario_steps;
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileSetupWindowDriver, ProfileSetupWizardDriver,
};
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::{ApplicationConfiguration, SystemTestApplication};
#[cfg(target_os = "windows")]
use keyringd_testkit::evidence_text;
use keyringd_testkit::scenario;

#[cfg(target_os = "windows")]
use keyringd_system_tests::environment::{NativeSystemTestEnvironment, SystemTestEnvironment};
#[cfg(target_os = "windows")]
use keyringd_system_tests::ssh::SshServers;

scenario! {
    group "Git for Windows SSH" {
        #[cfg(target_os = "windows")]
        case "Authenticates with the same certificate from Git for Windows and Windows OpenSSH after restart" {
            // UI-created Profile; real Git/Windows SSH and same-user sshd.
            // Test FIDO and software Device Key avoid physical hardware here.
            given "Profile created with the UI and where to connect to Git for Windows" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration {
                        test_fido_mode: Some(keyringd_core::test_support::TestFidoMode::Success),
                        ..Default::default()
                    })
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(
                    &mut environment,
                    "git-for-windows",
                )
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
                let trust = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "git-for-windows",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| identity.comment.ends_with(" device certificate"))
                    .expect("Agent did not publish the Device Certificate")
                    .to_openssh();
                let socket = environment
                    .application()
                    .git_for_windows_agent_socket()
                    .unwrap();
            }
            when "Git enclosed SSH authenticates through MSYS compatible sockets" {
                let mut server = environment.ssh_servers().start(&trust).unwrap();
                server
                    .set_client_agent(&environment.application().agent_endpoint().unwrap())
                    .unwrap();
                let output = server
                    .connect_with_git_for_windows(&identity, &socket)
                    .unwrap();
                output.assert_success();
                evidence_text!("Git for Windows SSH", &output.stderr);
                evidence_text!("Git for Windows sshd", server.server_log().unwrap());
            }
            when "Authenticate to the same Agent with Windows Standard SSH" {
                let mut server = environment.ssh_servers().start(&trust).unwrap();
                server
                    .set_client_agent(&environment.application().agent_endpoint().unwrap())
                    .unwrap();
                let output = server.connect(&identity).unwrap();
                output.assert_success();
                evidence_text!("Windows SSH alongside Git for Windows", &output.stderr);
                environment
                    .application()
                    .capture_screenshot("Git for Windows Profile")
                    .unwrap();
            }
            when "Restart Agent and reconnect from the included SSH with git with the same socket path" {
                environment.application().restart().unwrap();
                assert_eq!(
                    environment
                        .application()
                        .git_for_windows_agent_socket()
                        .unwrap(),
                    socket
                );
                let mut server = environment.ssh_servers().start(&trust).unwrap();
                server
                    .set_client_agent(&environment.application().agent_endpoint().unwrap())
                    .unwrap();
                let output = server
                    .connect_with_git_for_windows(&identity, &socket)
                    .unwrap();
                evidence_text!("Git for Windows SSH after restart", &output.stderr);
            }
            then "Agent signing and SSH authentication succeeds with the same certificate after reboot" {
                output.assert_success();
                evidence_text!(
                    "Git for Windows sshd after restart",
                    server.server_log().unwrap()
                );
            }
        }
    }
}
