use keyringd_test_usbip::authenticator_server::manager::{
    AuthenticatorServerManager, AuthenticatorServerOptions, AuthenticatorServerProcessError,
};
use std::time::Duration;

keyringd_testkit::scenario! {
    group "authenticator server manager" {
        case "Invalid inputs fail before touching the log" {
            async: tokio;
            verify "invalid inputs fail before touching the log" {
                let directory = tempfile::tempdir().unwrap();
                let jar = directory.path().join("server.jar");
                std::fs::write(&jar, "jar contents").unwrap();
                let result = AuthenticatorServerManager::start(AuthenticatorServerOptions::new(
                    jar.clone(),
                    jar.clone(),
                ))
                .await;
                assert!(matches!(
                    result,
                    Err(AuthenticatorServerProcessError::InvalidOptions(_))
                ));
                assert_eq!(std::fs::read_to_string(&jar).unwrap(), "jar contents");
                let log = directory.path().join("server.log");
                let mut options = AuthenticatorServerOptions::new(jar, log.clone());
                options.startup_timeout = Duration::ZERO;
                assert!(matches!(
                    AuthenticatorServerManager::start(options).await,
                    Err(AuthenticatorServerProcessError::InvalidOptions(_))
                ));
                assert!(!log.exists());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "authenticator server manager" {
        case "Occupied fixed ports fail before launching java" {
            async: tokio;
            verify "occupied fixed ports fail before launching java" {
                let directory = tempfile::tempdir().unwrap();
                let jar = directory.path().join("server.jar");
                let log = directory.path().join("server.log");
                std::fs::write(&jar, "jar").unwrap();
                let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
                let mut options = AuthenticatorServerOptions::new(jar, log.clone());
                options.usb_address = listener.local_addr().unwrap();
                options.java_executable = directory.path().join("does-not-exist");
                assert!(matches!(
                    AuthenticatorServerManager::start(options).await,
                    Err(AuthenticatorServerProcessError::Io {
                        operation: "reserve listener",
                        ..
                    })
                ));
                assert!(!log.exists());
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, path::Path};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn quote(path: &Path) -> String {
        format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
    }
    fn stub(directory: &Path, body: &str) -> AuthenticatorServerOptions {
        let java = directory.join("java-stub");
        std::fs::write(&java, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&java, std::fs::Permissions::from_mode(0o700)).unwrap();
        let jar = directory.join("server.jar");
        std::fs::write(&jar, "stub jar").unwrap();
        let mut options = AuthenticatorServerOptions::new(jar, directory.join("server.log"));
        options.java_executable = java;
        options.startup_timeout = Duration::from_secs(3);
        options
    }
    fn assert_reaped(pid: u32) {
        // The manager waits for the child, so it must no longer be a zombie.
        let exists = unsafe { libc::kill(pid as i32, 0) } == 0;
        assert!(!exists, "owned child {pid} survived cleanup");
    }
    async fn wait_file(path: &Path) -> String {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(text) = std::fs::read_to_string(path)
                && !text.is_empty()
            {
                return text;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "stub did not write {}",
                path.display()
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    keyringd_testkit::scenario! {
        group "authenticator server manager" {
            case "Early exit includes status and stdout stderr" {
                async: tokio;
                verify "early exit includes status and stdout stderr" {
                    let directory = tempfile::tempdir().unwrap();
                    let options = stub(
                        directory.path(),
                        "echo startup-out; echo startup-err >&2; exit 23",
                    );
                    match AuthenticatorServerManager::start(options).await {
                        Err(AuthenticatorServerProcessError::ExitedEarly { status, log_tail }) => {
                            assert_eq!(status.code(), Some(23));
                            assert!(log_tail.contains("startup-out"));
                            assert!(log_tail.contains("startup-err"));
                        }
                        _ => panic!("expected early exit"),
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "authenticator server manager" {
            case "Timeout and cancellation both reap the spawned child" {
                async: tokio;
                verify "timeout and cancellation both reap the spawned child" {
                    for cancel in [false, true] {
                        let directory = tempfile::tempdir().unwrap();
                        let pid_file = directory.path().join("pid");
                        let mut options = stub(
                            directory.path(),
                            &format!("echo $$ > {}; exec sleep 30", quote(&pid_file)),
                        );
                        if cancel {
                            assert!(
                                tokio::time::timeout(
                                    Duration::from_millis(150),
                                    AuthenticatorServerManager::start(options)
                                )
                                .await
                                .is_err()
                            );
                        } else {
                            options.startup_timeout = Duration::from_millis(150);
                            assert!(matches!(
                                AuthenticatorServerManager::start(options).await,
                                Err(AuthenticatorServerProcessError::StartupTimedOut { .. })
                            ));
                        }
                        let pid = wait_file(&pid_file).await.trim().parse().unwrap();
                        assert_reaped(pid);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "authenticator server manager" {
            case "Readiness requires usb and health then explicit stop and drop reap" {
                async: tokio;
                verify "readiness requires usb and health then explicit stop and drop reap" {
                    for explicit_stop in [false, true] {
                        let directory = tempfile::tempdir().unwrap();
                        let arguments = directory.path().join("args");
                        let pending_arguments = directory.path().join("args.tmp");
                        let options = stub(
                            directory.path(),
                            &format!(
                                "printf '%s\\n' \"$@\" > {}; mv {} {}; exec sleep 30",
                                quote(&pending_arguments),
                                quote(&pending_arguments),
                                quote(&arguments)
                            ),
                        );
                        let startup = tokio::spawn(AuthenticatorServerManager::start(options));
                        let args = wait_file(&arguments).await;
                        let port = |name: &str| {
                            args.lines()
                                .find_map(|line| line.strip_prefix(name))
                                .unwrap()
                                .parse::<u16>()
                                .unwrap()
                        };
                        let control = TcpListener::bind(("127.0.0.1", port("--control-port=")))
                            .await
                            .unwrap();
                        let usb = TcpListener::bind(("127.0.0.1", port("--usb-port=")))
                            .await
                            .unwrap();
                        assert_ne!(control.local_addr().unwrap(), usb.local_addr().unwrap());
                        let health = tokio::spawn(async move {
                            loop {
                                let (mut stream, _) = control.accept().await.unwrap();
                                let mut bytes = Vec::new();
                                while !bytes.ends_with(b"\r\n\r\n") {
                                    bytes.push(stream.read_u8().await.unwrap());
                                }
                                stream
                                    .write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n")
                                    .await
                                    .unwrap();
                            }
                        });
                        let usb_accept = tokio::spawn(async move {
                            loop {
                                let _ = usb.accept().await.unwrap();
                            }
                        });
                        let mut manager = startup.await.unwrap().unwrap();
                        manager.client().health().await.unwrap();
                        assert!(manager.hybrid_address().is_none());
                        let pid = manager.process_id().unwrap();
                        if explicit_stop {
                            manager.stop().unwrap();
                            manager.stop().unwrap();
                            assert!(manager.process_id().is_none());
                        }
                        drop(manager);
                        assert_reaped(pid);
                        health.abort();
                        usb_accept.abort();
                    }
                }
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "authenticator server manager" {
        case "Real server starts empty and releases ports after stop" {
            async: tokio;
            ignored: "requires a JDK";
            verify "real server starts empty and releases ports after stop" {
                let directory = tempfile::tempdir().unwrap();
                let jar = keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild::new(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .ancestors()
                        .nth(2)
                        .unwrap(),
                )
                .prepare()
                .unwrap();
                let mut server = AuthenticatorServerManager::start(AuthenticatorServerOptions::new(
                    jar,
                    directory.path().join("authenticator.log"),
                ))
                .await
                .unwrap();
                server.client().health().await.unwrap();
                assert!(server.client().authenticators().await.unwrap().is_empty());
                let key = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                key.delete().await.unwrap();
                let usb = server.usb_address();
                let control = server.control_address();
                server.stop().unwrap();
                server.stop().unwrap();
                assert!(std::net::TcpStream::connect_timeout(&usb, Duration::from_millis(100)).is_err());
                assert!(std::net::TcpStream::connect_timeout(&control, Duration::from_millis(100)).is_err());
            }
        }
    }
}
