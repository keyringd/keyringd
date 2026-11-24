use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::handler::NamedPipeHandler;

use crate::tests::support::completed_application::completed_application;

keyringd_testkit::scenario! {
    case "Discovers the Windows Agent through SSH_AUTH_SOCK" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        async: tokio;
        timeout: Duration::from_secs(30);

        given "a completed Agent served at a dedicated named pipe" {
            let pipe = format!(r"\\.\pipe\keyringd-rfc9987-{}", std::process::id());
            let application = completed_application(41, b"rfc9987-endpoint-discovery").await;
            let task = tokio::spawn(NamedPipeHandler::run_at(
                pipe.clone(),
                Arc::clone(&application),
                (),
                keyringd_ui_core::key_operation::KeyOperationCoordinator::channel().0,
            ));
        }
        when "OpenSSH resolves its Agent from SSH_AUTH_SOCK" {
            let deadline = Instant::now() + Duration::from_secs(5);
            let output = loop {
                let output = Command::new("ssh-add.exe")
                    .arg("-L")
                    .env("SSH_AUTH_SOCK", &pipe)
                    .output()
                    .await
                    .expect("ssh-add.exe should be executable because this test requires Windows OpenSSH to be installed");
                if output.status.success() || Instant::now() >= deadline {
                    break output;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            };
        }
        then "the client reaches keyringd and lists its identities" {
            assert!(
                output.status.success(),
                "ssh-add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("keyringd"));
            task.abort();
        }
    }
}
