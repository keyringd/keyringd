use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::support::completed_application::completed_application;
use crate::handler::NamedPipeHandler;

keyringd_testkit::scenario! {
    case "You can enumerate and sign Identity from the Windows OpenSSH Client" {
        async: tokio;
        timeout: Duration::from_secs(30);
        ignored: "Windows system test using the installed OpenSSH client";

        given "Prepare Agent_Pipe, Agent" {
            // Given a completed profile served through a dedicated Windows named pipe
            let agent_pipe = format!(r"\\.\pipe\keyringd-system-test-{}", std::process::id());
            let agent = completed_application(11, b"windows-system-test-key").await;
        }
        when "Start Named Pipe Server and get the Identity list from OpenSSH" {
            let pipe_server = tokio::spawn(NamedPipeHandler::run_at(
                agent_pipe.clone(),
                Arc::clone(&agent),
                (),
                keyringd_ui_core::key_operation::KeyOperationCoordinator::channel().0,
            ));
            // When native OpenSSH lists identities and signs two SSHSIG namespaces
            let identities = wait_for_identities(&agent_pipe);
        }
        then "You can enumerate and sign Identity from the Windows OpenSSH Client" {
            assert_success(&identities, "ssh-add -L");
            let certificate = String::from_utf8(identities.stdout)
                .unwrap()
                .lines()
                .find(|line| line.starts_with("sk-ecdsa-sha2-nistp256-cert-v01@openssh.com "))
                .unwrap()
                .to_owned();
            let directory = tempfile::tempdir().unwrap();
            let public_key = directory.path().join("device-cert.pub");
            let payload = directory.path().join("payload.txt");
            std::fs::write(&public_key, format!("{certificate}\n")).unwrap();
            std::fs::write(&payload, b"signed through the Windows named pipe\n").unwrap();
            sign_and_verify_sshsig(&public_key, &payload, "git", &agent_pipe);
            let release_payload = directory.path().join("release.txt");
            std::fs::write(&release_payload, b"namespace-independent SSHSIG\n").unwrap();
            sign_and_verify_sshsig(
                &public_key,
                &release_payload,
                "release@keyringd",
                &agent_pipe,
            );
            // Then both signatures have been accepted by native OpenSSH
                pipe_server.abort();
        }

    }
}

fn wait_for_identities(agent_pipe: &str) -> std::process::Output {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let output = openssh("ssh-add", &["-L"], agent_pipe);
        if output.status.success() {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the Windows named-pipe agent: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn sign_and_verify_sshsig(
    public_key: &std::path::Path,
    payload: &std::path::Path,
    namespace: &str,
    agent_pipe: &str,
) {
    let signature = Command::new("ssh-keygen")
        .args(["-Y", "sign", "-U", "-f"])
        .arg(public_key)
        .args(["-n", namespace])
        .arg(payload)
        .env("SSH_AUTH_SOCK", agent_pipe)
        .output()
        .expect("ssh-keygen.exe should be executable because this test requires Windows OpenSSH to be installed");
    assert_success(&signature, "ssh-keygen -Y sign");

    let signature_path = payload.with_extension(format!(
        "{}.sig",
        payload.extension().unwrap_or_default().to_string_lossy()
    ));
    assert!(
        signature_path.is_file(),
        "ssh-keygen did not produce an SSHSIG file"
    );

    let mut verification = Command::new("ssh-keygen")
        .args(["-Y", "check-novalidate", "-n", namespace, "-s"])
        .arg(&signature_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ssh-keygen.exe should be executable because this test requires Windows OpenSSH to be installed");
    verification
        .stdin
        .take()
        .unwrap()
        .write_all(&std::fs::read(payload).unwrap())
        .unwrap();
    let verification = verification.wait_with_output().unwrap();
    assert_success(&verification, "ssh-keygen -Y check-novalidate");
}

fn openssh(program: &str, arguments: &[&str], agent_pipe: &str) -> std::process::Output {
    Command::new(program)
        .args(arguments)
        .env("SSH_AUTH_SOCK", agent_pipe)
        .output()
        .unwrap_or_else(|error| panic!("failed to start Windows {program}: {error}"))
}

fn assert_success(output: &std::process::Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
