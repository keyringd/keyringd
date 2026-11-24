#[cfg(windows)]
use crate::scenario_steps;
#[cfg(windows)]
use keyringd_system_tests::application::ApplicationConfiguration;
#[cfg(windows)]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(windows)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(windows)]
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWizardDriver;
#[cfg(target_os = "windows")]
use keyringd_system_tests::application::features::profile_setup::{
    CompletionPageDriver, ProfileFormPageDriver, ProfileSetupInteractionPageDriver,
    ProfileSetupQrViewDriver,
};
#[cfg(windows)]
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
use keyringd_system_tests::environment::SystemTestEnvironment;
#[cfg(windows)]
use keyringd_system_tests::ssh::SshServers;
#[cfg(target_os = "linux")]
use keyringd_testkit::evidence_log;
#[cfg(target_os = "windows")]
use keyringd_testkit::evidence_text;
use keyringd_testkit::scenario;

scenario! {
    group "Service Management / SSH Client Configuration" {
        #[cfg(target_os = "linux")]
        case "Sets, expands, and unsets SSH_AUTH_SOCK through systemd" {
            given "Open the profile list" {
                let mut environment =
                    keyringd_system_tests::environment::NativeSystemTestEnvironment::new()
                        .unwrap();
                environment.application().start(Default::default()).unwrap();
            }
            when "Press the Configure SSH Clients button in Profiles" {
                let setup = environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap();
            }
            then "The Configure SSH Clients to Use Keyringd screen appears" {
                setup.capture_screenshot("Configure SSH Clients to Use Keyringd").unwrap();
            }
            when "Check and run OpenSSH settings" {
                let confirmation = setup.open_confirmation().unwrap();
                confirmation
                    .capture_screenshot("OpenSSH configuration confirmation")
                    .unwrap();
                confirmation.confirm().unwrap();
            }
            then "Settings are saved in Environment.D" {
                let config = environment.testbed().ssh_client_configuration_text().unwrap();
                assert_eq!(
                    config,
                    "SSH_AUTH_SOCK=${XDG_RUNTIME_DIR}/keyringd-agent.sock\n"
                );
                evidence_log!("Saved SSH_AUTH_SOCK configuration", &config);
                // Exercise systemd's actual environment generator, including
                // expansion of XDG_RUNTIME_DIR, without changing the test session.
                let generated = environment
                    .testbed()
                    .generated_ssh_agent_environment()
                    .unwrap();
                assert!(generated.status.success(), "{}", generated.stderr);
                let endpoint = environment.application().agent_endpoint().unwrap();
                assert!(
                    generated
                        .stdout
                        .lines()
                        .any(|line| line == format!("SSH_AUTH_SOCK={endpoint}")),
                    "{}",
                    generated.stdout
                );
                evidence_log!("systemd generated environment", &generated.stdout);
            }
            when "Remove settings from the same screen" {
                let confirmation = setup.open_removal_confirmation().unwrap();
                confirmation.confirm().unwrap();
            }
            then "The configuration file is deleted and returns to CONFIGURE" {
                assert!(!environment.testbed().ssh_client_configuration_exists().unwrap());
                setup
                    .capture_screenshot("SSH_AUTH_SOCK unconfigured")
                    .unwrap();
            }
        }
    }
}

scenario! {
    group "Service Management / SSH Client Configuration" {
        #[cfg(target_os = "macos")]
        case "Leaves SSH client selection unchanged when configuration is canceled" {
            given "Applications launched with dedicated Home" {
                let mut environment =
                    keyringd_system_tests::environment::NativeSystemTestEnvironment::new()
                        .unwrap();
                let application = environment.application();
                application.start(Default::default()).unwrap();
                let page = application.ssh_client_configuration_window().unwrap();
            }
            when "Cancel the confirmation of the settings" {
                let confirmation = page.open_confirmation().unwrap();
                confirmation
                    .capture_screenshot("SSH_AUTH_SOCK confirmation")
                    .unwrap();
                confirmation.cancel().unwrap();
            }
            then "The SSH agent selection remains disabled" {
                assert!(!application.ssh_agent_selection_enabled().unwrap());
                page.capture_screenshot("SSH agent setup").unwrap();
                page.close().unwrap();
            }
        }
    }
}

scenario! {
    group "Service Management / SSH Client Configuration" {
        #[cfg(target_os = "windows")]
        case "Copies the Git for Windows configuration without changing Windows settings" {
            given "Configure SSH Clients screen using dedicated setting area" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
                environment
                    .application()
                    .profiles_window()
                    .unwrap()
                    .open_ssh_client_configuration()
                    .unwrap();
            }
            when "Open and cancel the settings guide for Git for Windows" {
                let page = environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .open_git_for_windows_setting()
                    .unwrap();
                page.capture_screenshot("Git for Windows SSH setup")
                    .unwrap();
                page.cancel().unwrap();
                assert_eq!(environment.application().configured_agent().unwrap(), None);
            }
            when "Copy the set line to the clipboard" {
                let page = environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .open_git_for_windows_setting()
                    .unwrap();
                assert_eq!(
                    page.copy().unwrap(),
                    "export SSH_AUTH_SOCK=\"$(cygpath -u \"$LOCALAPPDATA\")/keyringd/agent.sock\""
                );
            }
            then "Windows SSH settings remain unchanged" {
                assert_eq!(environment.application().configured_agent().unwrap(), None);
                environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .close()
                    .unwrap();
                environment
                    .application()
                    .application_menu()
                    .unwrap()
                    .exit()
                    .unwrap();
                environment.application().wait_exited().unwrap();
            }
        }
        #[cfg(target_os = "windows")]
        case "Sets SSH_AUTH_SOCK, authenticates with a new OpenSSH process, and unsets it" {
            // UI Scope: Windows products Tray→Configure SSH Clients→Windows SSH settings. Write ssh_auth_sock to the dedicated HKCU key with the same processing as the product, pass the value to the new OpenSSH process and authenticate SSH. User's Environment key and environment change Broadcast are not included.
            given "PROFILE created with product TRAY and UI" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                let _security_key = environment
                    .authenticators()
                    .create_security_key(Default::default())
                    .unwrap();
                environment
                    .application()
                    .start(ApplicationConfiguration::default())
                    .unwrap();
                environment
                    .application()
                    .profile_setup_window()
                    .open()
                    .unwrap();
                scenario_steps::profile_setup::form::submit_creation(&mut environment, "menu-ssh")
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
                environment
                    .application()
                    .profile_setup_window()
                    .wizard()
                    .unwrap()
                    .completion_page()
                    .unwrap()
                    .done()
                    .unwrap();
                scenario_steps::profile_management::profile_list::assert_visible(
                    &environment.application().profiles_window().unwrap(),
                    "menu-ssh",
                )
                .unwrap();
                assert_eq!(environment.application().configured_agent().unwrap(), None);
                let trust = scenario_steps::profile_management::public_keys::copy_device_key(
                    &mut environment,
                    "menu-ssh",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|i| i.algorithm == "ecdsa-sha2-nistp256")
                    .unwrap();
            }
            when "Open Configure SSH Clients from the Tray menu" {
                let menu = environment.application().application_menu().unwrap();
                menu.capture_screenshot("Tray menu with Configure SSH Clients")
                    .unwrap();
                let setup = menu.open_ssh_client_configuration().unwrap();
                setup
                    .capture_screenshot("Configure SSH Clients (Windows)")
                    .unwrap();
            }
            when "Cancel the confirmation of Windows SSH settings" {
                let confirmation = environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .open_windows_configuration()
                    .unwrap();
                confirmation
                    .capture_screenshot("Windows SSH configuration confirmation")
                    .unwrap();
                confirmation.cancel().unwrap();
            }
            then "Settings are not written" {
                assert_eq!(environment.application().configured_agent().unwrap(), None);
            }
            when "Check and run Windows SSH settings" {
                let confirmation = environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .open_windows_configuration()
                    .unwrap();
                let result = confirmation.configure().unwrap();
                assert!(
                    result
                        .content()
                        .unwrap()
                        .iter()
                        .any(|message| message == "Restart your applications to apply the setting")
                );
                result.capture_screenshot("Windows SSH configured").unwrap();
                result.close().unwrap();
                let agent = environment
                    .application()
                    .configured_agent()
                    .unwrap()
                    .unwrap();
                environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .close()
                    .unwrap();
            }
            then "New OpenSSH processes can be authenticated with saved Agent settings" {
                assert_eq!(agent, environment.application().agent_endpoint().unwrap());
                let mut server = environment.ssh_servers().start(&trust).unwrap();
                server
                    .set_client_agent(&environment.application().agent_endpoint().unwrap())
                    .unwrap();
                let output = server
                    .connect_with_agent_environment(&identity.to_openssh(), &agent)
                    .unwrap();
                output.assert_success();
                evidence_text!(
                    "SSH using configured SSH_AUTH_SOCK",
                    server.server_log().unwrap()
                );
                environment
                    .application()
                    .application_menu()
                    .unwrap()
                    .open_ssh_client_configuration()
                    .unwrap();
                environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .unconfigure()
                    .unwrap();
                assert_eq!(environment.application().configured_agent().unwrap(), None);
                environment
                    .application()
                    .ssh_client_configuration_window()
                    .unwrap()
                    .close()
                    .unwrap();
                environment
                    .application()
                    .application_menu()
                    .unwrap()
                    .exit()
                    .unwrap();
                environment.application().wait_exited().unwrap();
            }
        }
    }
}

#[cfg(target_os = "macos")]
scenario! {
    group "Service Management / SSH Client Configuration" {
        case "Switches existing client paths and restores the original socket without a new login" {
            given "Start the app with a private original SSH agent socket" {
                use keyringd_system_tests::environment::NativeSystemTestEnvironment;
                use std::os::unix::fs::MetadataExt;
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
                environment.application().start(Default::default()).unwrap();
                let original = environment.application().original_agent_socket();
                let original_inode = std::fs::metadata(&original).unwrap().ino();
                let socket = environment.application().agent_socket().unwrap();
            }
            when "Configure Keyringd in the application" {
                let page = environment.application().ssh_client_configuration_window().unwrap();
                page.open_confirmation().unwrap().confirm().unwrap();
                page.close().unwrap();
            }
            then "Existing clients reach the running Keyringd socket through the unchanged path" {
                assert!(environment.application().ssh_agent_selection_enabled().unwrap());
                assert_eq!(std::fs::read_link(&original).unwrap(), std::path::PathBuf::from(&socket));
                let mut backup = original.as_os_str().to_owned();
                backup.push(".keyringd-original");
                let backup = std::path::PathBuf::from(backup);
                assert_eq!(std::fs::metadata(&backup).unwrap().ino(), original_inode);
                std::os::unix::net::UnixStream::connect(&original).unwrap();
            }
            when "Restart Keyringd and unconfigure in the same session" {
                // Restore the OS endpoint while retaining the persisted choice.
                // Restart must read setting.json and apply that choice again.
                let mut backup = original.as_os_str().to_owned();
                backup.push(".keyringd-original");
                std::fs::rename(std::path::PathBuf::from(backup), &original).unwrap();
                environment.application().restart().unwrap();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while std::fs::read_link(&original).ok().as_deref() != Some(std::path::Path::new(&socket)) {
                    assert!(std::time::Instant::now() < deadline, "startup did not apply the saved SSH agent selection");
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                assert!(environment.application().ssh_agent_selection_enabled().unwrap());
                let page = environment.application().ssh_client_configuration_window().unwrap();
                page.open_confirmation().unwrap().confirm().unwrap();
                page.close().unwrap();
            }
            then "The original listening socket is restored and the dedicated socket still works" {
                assert_eq!(std::fs::metadata(&original).unwrap().ino(), original_inode);
                assert!(!std::fs::symlink_metadata(&original).unwrap().file_type().is_symlink());
                assert!(!environment.application().ssh_agent_selection_enabled().unwrap());
                std::os::unix::net::UnixStream::connect(&original).unwrap();
                std::os::unix::net::UnixStream::connect(&socket).unwrap();
            }
        }
    }
}
