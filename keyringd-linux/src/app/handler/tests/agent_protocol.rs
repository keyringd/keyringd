use crate::system::test_support::PlatformFixture;

use super::TestAgentServer;

fn write_string(buffer: &mut Vec<u8>, value: &[u8]) {
    buffer.extend_from_slice(&u32::try_from(value.len()).unwrap().to_be_bytes());
    buffer.extend_from_slice(value);
}

fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

keyringd_testkit::scenario! {
    case "Completed Profile Exposes Certificate, Device Key, and Passkey Identity Sequentially" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Start an agent with a Profile with a Device Key and Passkey" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let server = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));
            let mut stream = server.connect().await;

        }
        when "Call TestAgentServer::send_request" {
            let response = TestAgentServer::send_request(&mut stream, &[11]).await;

        }
        then "Returns the identity of each algorithm in the order of Certificate, Device Key, Passkey" {
            assert_eq!(response[0], 12);
            assert_eq!(u32::from_be_bytes(response[1..5].try_into().unwrap()), 3);
            let (cert_blob, rest) = TestAgentServer::parse_string(&response[5..]);
            let (cert_type, _) = TestAgentServer::parse_string(cert_blob);
            assert_eq!(cert_type, b"ecdsa-sha2-nistp256-cert-v01@openssh.com");
            let (_, rest) = TestAgentServer::parse_string(rest);
            let (device_blob, rest) = TestAgentServer::parse_string(rest);
            let (device_type, _) = TestAgentServer::parse_string(device_blob);
            assert_eq!(device_type, b"ecdsa-sha2-nistp256");
            let (_, rest) = TestAgentServer::parse_string(rest);
            let (passkey_blob, _) = TestAgentServer::parse_string(rest);
            let (passkey_type, _) = TestAgentServer::parse_string(passkey_blob);
            assert_eq!(passkey_type, b"sk-ecdsa-sha2-nistp256@openssh.com");
        }

    }
}

keyringd_testkit::scenario! {
    case "Completed Profile Signs SSHSIG with Device Key" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare inputs and initial states to validate that \"Complete Profile Signs to SSHSIG with Device Key\"" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let server = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));
            let mut stream = server.connect().await;
            let identities = TestAgentServer::send_request(&mut stream, &[11]).await;
            let (cert_blob, _) = TestAgentServer::parse_string(&identities[5..]);

            let mut sshsig = b"SSHSIG".to_vec();
            write_string(&mut sshsig, b"integration-test");
            write_string(&mut sshsig, b"");
            write_string(&mut sshsig, b"sha256");
            write_string(&mut sshsig, &[0x42; 32]);
            let mut request = vec![13];
            write_string(&mut request, cert_blob);
            write_string(&mut request, &sshsig);
            write_u32(&mut request, 0);

        }
        when "Call TestAgentServer::send_request" {
            let response = TestAgentServer::send_request(&mut stream, &request).await;

        }
        then "Check the Results and Side Effects of \"Completed Profile Signs SSHSIG with Device Key\"" {
            assert_eq!(response[0], 14);
        }

    }
}

keyringd_testkit::scenario! {
    case "Completed Profile Signs Directly with Device Key" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare PlatformFixture/TestAgentServer" {
            // Given the direct Device Key identity exposed after the certificate
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let server = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));
            let mut stream = server.connect().await;
            let identities = TestAgentServer::send_request(&mut stream, &[11]).await;
            let (_, rest) = TestAgentServer::parse_string(&identities[5..]);
            let (_, rest) = TestAgentServer::parse_string(rest);
            let (device_blob, _) = TestAgentServer::parse_string(rest);
        }
        when "Call WRITE_STRING" {
            let mut sshsig = b"SSHSIG".to_vec();
            write_string(&mut sshsig, b"integration-test");
            write_string(&mut sshsig, b"");
            write_string(&mut sshsig, b"sha256");
            write_string(&mut sshsig, &[0x24; 32]);
            let mut request = vec![13];
            write_string(&mut request, device_blob);
            write_string(&mut request, &sshsig);
            write_u32(&mut request, 0);
            // When it is asked to sign, the same TPM-backed Device Key is used.
            let response = TestAgentServer::send_request(&mut stream, &request).await;
        }
        then "Completed Profile Signs Directly with Device Key" {
            assert_eq!(response[0], 14);
        }

    }
}
use std::sync::Arc;
