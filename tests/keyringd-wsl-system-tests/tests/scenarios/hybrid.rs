use crate::support::{IdentityKind, approve_hybrid_signing, prepare_hybrid_ssh};
use keyringd_system_tests::application::features::navigation::windows::UiApplication;
#[cfg(windows)]
use keyringd_system_tests::application::features::qr_prompt_window::QrPromptWindowDriver;
use keyringd_testkit::scenario;
use keyringd_wsl_system_tests::WslFixture;

scenario! {
    case "Reconnects using the Device Certificate through the WSL 2 proxy" {
        given "Profile created in Windows and real Proxy SSHD in WSL2" {
            let (mut app, driver, profile, _phone) =
                prepare_hybrid_ssh("wsl-profile", IdentityKind::DeviceCertificate).unwrap();
            let fixture =
                WslFixture::start(&profile.trust_entry, &profile.identity, &mut app).unwrap();
        }
        when "Enumerate Identity for Windows Agents from WSL2" {
            let identities = fixture.identities().unwrap();
            app.wait_socket_activation().unwrap();
        }
        then "You can get the Windows Agent Identity from WSL2" {
            assert!(identities.lines().any(|line| {
                line.split_whitespace()
                    .take(2)
                    .eq(profile.identity.split_whitespace().take(2))
            }));
            keyringd_testkit::evidence_text!("WSL agent identities", identities);
        }
        when "Connect SSH from WSL2 using Windows Agent" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let output = client.finish().unwrap();
        }
        then "SSH authentication succeeds with public key" {
            fixture.assert_connected(&output).unwrap();
            keyringd_testkit::evidence_text!(
                "WSL SSH client",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        when "Connect SSH from WSL2 using Windows Agent" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let output = client.finish().unwrap();
        }
        then "SSH authentication succeeds with public key" {
            fixture.assert_connected(&output).unwrap();
            keyringd_testkit::evidence_text!(
                "WSL SSH client",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        then "Windows Agent's signature public key authentication succeeds" {
            keyringd_testkit::evidence_text!("WSL sshd and proxy", fixture.logs().unwrap());
            let ui = UiApplication::new(&driver, app.pid());
            ui.capture_screenshot("Windows profile").unwrap();
        }
    }

    case "Authenticates with the Device Key through the WSL 2 proxy" {
        given "WSL2 SSHD to Trust Profile and Device Key created in Windows" {
            let (mut app, driver, profile, _phone) =
                prepare_hybrid_ssh("wsl-profile", IdentityKind::DeviceKey).unwrap();
            let fixture =
                WslFixture::start(&profile.trust_entry, &profile.identity, &mut app).unwrap();
        }
        when "OpenSSH in WSL2 asks for signature via Windows Bridge" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let output = client.finish().unwrap();
        }
        then "Public Key Authentication and Remote Command Successful" {
            fixture.assert_connected(&output).unwrap();
            keyringd_testkit::evidence_text!(
                "WSL SSH client",
                String::from_utf8_lossy(&output.stderr)
            );
            keyringd_testkit::evidence_text!("WSL sshd and proxy", fixture.logs().unwrap());
            let ui = UiApplication::new(&driver, app.pid());
            ui.capture_screenshot("Windows profile").unwrap();
        }
    }

    case "Authenticates through the WSL 2 proxy after Windows approves Passkey use" {
        given "WSL2 SSHD to Trust Profiles Created in Windows and Passkey" {
            let (mut app, driver, profile, phone) =
                prepare_hybrid_ssh("wsl-profile", IdentityKind::Passkey).unwrap();
            let fixture =
                WslFixture::start(&profile.trust_entry, &profile.identity, &mut app).unwrap();
        }
        when "Approve a Linux OpenSSH signature request on a Windows screen" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let ui = UiApplication::new(&driver, app.pid());
            approve_hybrid_signing(&app, &ui, &phone).unwrap();
            let output = client.finish().unwrap();
        }
        then "Passkey's public key authentication succeeds" {
            fixture.assert_connected(&output).unwrap();
            keyringd_testkit::evidence_text!(
                "WSL SSH client",
                String::from_utf8_lossy(&output.stderr)
            );
            keyringd_testkit::evidence_text!("WSL sshd and proxy", fixture.logs().unwrap());
            let ui = UiApplication::new(&driver, app.pid());
            ui.capture_screenshot("Windows profile").unwrap();
        }
    }

    case "Rejects a WSL 2 signing request in Windows and approves the next connection" {
        given "WSL2 SSHD Trust Windows Profile and Passkey" {
            let (mut app, driver, profile, phone) =
                prepare_hybrid_ssh("wsl-profile", IdentityKind::Passkey).unwrap();
            let fixture =
                WslFixture::start(&profile.trust_entry, &profile.identity, &mut app).unwrap();
        }
        when "Cancel WSL2 SSH Authentication Request" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let ui = UiApplication::new(&driver, app.pid());
            ui.qr_prompt_window().cancel().unwrap();
            let rejected = client.finish().unwrap();
        }
        then "The HYBRID signature screen closes" {
            assert_eq!(rejected.status.code(), Some(255));
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("Permission denied"));
            assert!(!fixture.logs().unwrap().contains("Accepted publickey"));
            keyringd_testkit::evidence_text!(
                "Rejected WSL SSH",
                String::from_utf8_lossy(&rejected.stderr)
            );
            ui.qr_prompt_window().wait_closed().unwrap();
        }
        when "Approve the following WSL2 SSH authentication in Hybrid" {
            let client = fixture.start_client().unwrap();
            app.wait_socket_activation().unwrap();
            let ui = UiApplication::new(&driver, app.pid());
            approve_hybrid_signing(&app, &ui, &phone).unwrap();
            let accepted = client.finish().unwrap();
        }
        then "Rejected connections are not authenticated and the next connection succeeds" {
            fixture.assert_connected(&accepted).unwrap();
            keyringd_testkit::evidence_text!("WSL sshd and proxy", fixture.logs().unwrap());
        }
    }
}
