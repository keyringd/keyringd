#[cfg(unix)]
use crate::scenario_steps;
#[cfg(unix)]
use keyringd_system_tests::application::SystemTestApplication;
#[cfg(unix)]
use keyringd_system_tests::environment::{
    NativeSocketActivationFixture, NativeSystemTestEnvironment, SocketActivationDriver,
    SystemTestEnvironment,
};
#[cfg(unix)]
use keyringd_system_tests::ssh::{SshServers, SshSession, SshTestServer};
use keyringd_testkit::scenario;
#[cfg(unix)]
use keyringd_testkit::{evidence_log, evidence_text};

scenario! {
    group "Service Management / Socket Activation" {
        #[cfg(unix)]
        case "Starts the service on socket connection and reuses the process" {
            given "Prepare socket startup with service stopped" {
                let service = NativeSocketActivationFixture::prepare().unwrap();
                let socket = service.socket_path().unwrap();
            }
            then "There is no service process before the connection" {
                assert_eq!(service.process_id().unwrap(), None);
            }
            when "Connect to the socket" {
                let first_connection = std::os::unix::net::UnixStream::connect(&socket).unwrap();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                let pid = loop {
                    if let Some(pid) = service.process_id().unwrap() {
                        break pid;
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "service did not start after socket connection"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(25));
                };
            }
            then "The service process is running" {
                case_execution.attach_text(
                    "Activated service",
                    format!("PID={pid}\nSocket={}", socket.display()),
                );
            }
            when "Continue to connect to the same socket" {
                let second_connection = std::os::unix::net::UnixStream::connect(&socket).unwrap();
            }
            then "The same service process continues to work" {
                assert_eq!(service.process_id().unwrap(), Some(pid));
                drop((first_connection, second_connection));
                drop(service);
                #[cfg(target_os = "linux")]
                assert!(!socket.exists(), "the fixture left its activation socket listening");
            }
        }

        #[cfg(unix)]
        case "Authenticates over SSH through socket activation with a profile created in the GUI" {
            skip_if: !keyringd_system_tests::authenticator::Authenticators::usb_available()
                => "requires the OS Security Key test environment";
            given "Prepare an SSH server that trusts the Profile created in the GUI and the CA" {
                let (mut environment, _key) =
                    scenario_steps::profile_setup::profile_fixture::create::<
                        NativeSystemTestEnvironment,
                    >(&["socket-activation-profile"])
                    .unwrap();
                let ca = scenario_steps::profile_management::public_keys::copy_ca(
                    &mut environment,
                    "socket-activation-profile",
                )
                .unwrap();
                let identity = environment
                    .application()
                    .agent_identities()
                    .unwrap()
                    .into_iter()
                    .find(|identity| {
                        identity.comment
                            == "keyringd socket-activation-profile device certificate"
                    })
                    .unwrap();
                let server = environment.ssh_servers().start(&ca).unwrap();
                evidence_text!("CA trust entry", &ca);
            }
            when "Stop the application and prepare a socket to start with a saved Profile" {
                let service = environment.prepare_application_socket_activation().unwrap();
                let socket = service
                    .socket_path()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
            }
            then "There is no service process before SSH connection" {
                assert_eq!(service.process_id().unwrap(), None);
            }
            when "Make the first SSH connection using a socket" {
                let first = server
                    .start_client(&socket, &identity, "printf first-connection")
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "The service has started and can be authenticated with the profile certificate created in the GUI" {
                first.assert_authenticated(&identity);
                assert_eq!(first.stdout, "first-connection");
                let pid = service
                    .process_id()
                    .unwrap()
                    .expect("socket-activated service is not running");
                evidence_log!("first SSH stderr", &first.stderr);
                evidence_text!("Activated service", format!("PID={pid}\nSocket={socket}"));
            }
            when "Make a second SSH connection using the same socket" {
                let server = environment.ssh_servers().start(&ca).unwrap();
                let second = server
                    .start_client(&socket, &identity, "printf second-connection")
                    .unwrap()
                    .finish()
                    .unwrap();
            }
            then "You can authenticate the certificate for the second time in the same service process" {
                second.assert_authenticated(&identity);
                assert_eq!(second.stdout, "second-connection");
                assert_eq!(service.process_id().unwrap(), Some(pid));
                evidence_log!("second SSH stderr", &second.stderr);
                evidence_log!("sshd log", &second.server_log);
            }
        }

        #[cfg(target_os = "linux")]
        case "Uses the isolated container environment for the installed GUI and systemd services" {
            given "Prepare a quarantine environment within the container" {
                let mut environment = NativeSystemTestEnvironment::new().unwrap();
            }
            when "Launch an installed application" {
                environment.application().start(Default::default()).unwrap();
            }
            then "Product processes and destinations are isolated in the test environment" {
                environment
                    .testbed()
                    .assert_application_is_isolated()
                    .unwrap();
                let journal = environment.testbed().agent_journal().unwrap();
                evidence_log!("agent journal", &journal);
                environment.application().stop().unwrap();
            }
        }
    }
}
