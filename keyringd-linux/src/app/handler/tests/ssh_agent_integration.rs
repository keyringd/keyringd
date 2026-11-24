use crate::system::test_support::PlatformFixture;
use base64::Engine;
use keyringd_core::model::ssh_trust::{
    SshCertificateAuthorityTrustEntry, SshDeviceKeyTrustEntry, SshPasskeyTrustEntry,
};
use keyringd_core::persistence::FileProfileRepository;
use keyringd_core::persistence::ProfileRepository;
use std::sync::Arc;

use super::{TestAgentServer, TestSshd};

keyringd_testkit::scenario! {
    case "Authenticate multiple SSH connections with Certificate Authority" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare input and initial state to validate \"Certificate Authority authentication multiple times\"" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let profile = fixture.application.profiles()[0].clone();
            let authorized = SshCertificateAuthorityTrustEntry::from_profile(&profile);
            let sshd = TestSshd::start(authorized.as_ref());
            let agent = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));

        }
        when "Call Agent.Socket_Path" {
            let mut outputs = Vec::new();
            for marker in ["first-connection", "second-connection"] {
                let output = sshd
                    .run_ssh(agent.socket_path(), &format!("echo {marker}"))
                    .await;
                outputs.push((marker, output));
            }
        }
        then "Check the Results and Side Effects of \"Authenticating Multiple SSH Connections with Certificate Authority\"" {
            for (marker, output) in outputs {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains(marker),
                    "SSH failed: {stderr}"
                );
                assert!(
                    stderr.contains("ECDSA-CERT"),
                    "expected certificate: {stderr}"
                );
            }
        }

    }
}

keyringd_testkit::scenario! {
    case "Authenticate SSH Connections Directly with Passkey" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare inputs and initial states to validate \"Direct Authenticating SSH Connections with Passkey\"" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let profile = fixture.application.profiles()[0].clone();
            let passkey = SshPasskeyTrustEntry::from_profile(&profile);
            let sshd = TestSshd::start(&format!("no-touch-required {passkey}"));
            let agent = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));

        }
        when "Call sshd.run_ssh" {
            let output = sshd.run_ssh(agent.socket_path(), "echo e2e-fido-ok").await;
            let stderr = String::from_utf8_lossy(&output.stderr);

        }
        then "Check the results and side effects of \"Authenticating SSH connections directly with Passkey\"" {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("e2e-fido-ok"),
                "SSH failed: {stderr}"
            );
            assert!(stderr.contains("ECDSA-SK"), "expected ECDSA-SK: {stderr}");
        }

    }
}

keyringd_testkit::scenario! {
    case "Authenticate directly with Device Key without Passkey operation" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Prepare inputs and initial states to validate \"Direct authentication with Device Key without Passkey operation\"" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let profile = fixture.application.profiles()[0].clone();
            let device_key = SshDeviceKeyTrustEntry::from_profile(&profile);
            let sshd = TestSshd::start(device_key.as_ref());
            let agent = TestAgentServer::start(fixture.data_dir(), Arc::clone(&fixture.application));

        }
        when "Call Agent.Socket_Path" {
            let output = sshd
                .run_ssh(agent.socket_path(), "echo e2e-device-ok")
                .await;
            let stderr = String::from_utf8_lossy(&output.stderr);

        }
        then "Check for Results and Side Effects of \"Direct Authentication with Device Key Without Passkey Operation\"" {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("e2e-device-ok"),
                "SSH failed: {stderr}"
            );
            assert!(stderr.contains("ECDSA SHA256:"), "expected ECDSA: {stderr}");
        }

    }
}

keyringd_testkit::scenario! {
    case "Omits expired certificates and leaves renewal out of SSH requests" {
        async: tokio;
        timeout: std::time::Duration::from_secs(10);

        given "Saved Profile Holding Expired Certificates" {
            let fixture = PlatformFixture::new().await;
            fixture.complete_initial_profile().await;
            let authorized = SshCertificateAuthorityTrustEntry::from_profile(&fixture.application.profiles()[0]);
            let store = FileProfileRepository::new(Arc::new(crate::system::persistence::LinuxAtomicFile::new(&fixture.data_dir().join("profiles.json"))));
            let mut records = store.load().unwrap();
            let certificate = records[0].device_key().certificate().clone();
            // Enumeration checks validity before use; signature verification is not needed
            // for this expired fixture, which must never be offered to the SSH client.
            let mut body = certificate.body().clone();
            body.fields_mut().valid_after = 0;
            body.fields_mut().valid_before = 1;
            let expired = body.prepare().unwrap().with_signature(certificate.signature().clone());
            let mut stored = serde_json::to_value(&records[0]).unwrap();
            stored["deviceKey"]["certificate"] = serde_json::Value::String(
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(expired.to_bytes().as_slice()),
            );
            records[0] = serde_json::from_value(stored).unwrap();
            store.save(&keyringd_core::persistence::ProfilesDocument { schema_version: keyringd_core::persistence::PROFILES_SCHEMA_VERSION, profiles: records.clone() }).unwrap();
            let application = fixture.reopen_application().await;
            let sshd = TestSshd::start(authorized.as_ref());
            let agent = TestAgentServer::start(fixture.data_dir(), application);
        }
        when "Request an Identity list and SSH signature from the Agent" {
            let mut stream = agent.connect().await;
            let identities = TestAgentServer::send_request(&mut stream, &[11]).await;
            let after = sshd.run_ssh(agent.socket_path(), "echo expired-certificate").await;
        }
        then "The only public key is the direct public key and the certificate is not renewed" {
            assert_eq!(identities[0], 12);
            assert_eq!(u32::from_be_bytes(identities[1..5].try_into().unwrap()), 2);
            let mut remaining = &identities[5..];
            for _ in 0..2 {
                let (key, rest) = TestAgentServer::parse_string(remaining);
                let (_, rest) = TestAgentServer::parse_string(rest);
                let (algorithm, _) = TestAgentServer::parse_string(key);
                assert!(!algorithm.ends_with(b"-cert-v01@openssh.com"));
                remaining = rest;
            }
            assert!(!after.status.success(), "SSH unexpectedly accepted an expired certificate");
            assert_eq!(store.load().unwrap()[0].device_key().certificate().to_key_blob(), records[0].device_key().certificate().to_key_blob());
        }
    }
}
