use keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild;

keyringd_testkit::scenario! {
    group "authenticator server build" {
        case "Rejects a missing gradle project" {
            verify "rejects a missing gradle project" {
                let workspace = tempfile::tempdir().unwrap();
                let error = AuthenticatorServerBuild::new(workspace.path())
                    .prepare()
                    .unwrap_err();
                assert!(error.to_string().contains("Gradle project is missing"));
                assert!(!workspace.path().join("target").exists());
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};

    fn project(workspace: &Path, script: &str) {
        let project = workspace.join("tests/keyringd-test-usbip/authenticator-server");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("build.gradle.kts"), "first inputs").unwrap();
        let wrapper = project.join("gradlew");
        fs::write(&wrapper, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
        fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }

    keyringd_testkit::scenario! {
        group "authenticator server build" {
            case "Prepares from a workspace with spaces and rebuilds existing outputs" {
                verify "prepares from a workspace with spaces and rebuilds existing outputs" {
                    let workspace = tempfile::Builder::new()
                        .prefix("workspace with spaces ")
                        .tempdir()
                        .unwrap();
                    let java_home = workspace.path().join("JDK with spaces");
                    project(
                        workspace.path(),
                        r#"
            mkdir -p ../../../target/authenticator-server/libs
            printf '%s' "$JAVA_HOME" > ../../../target/authenticator-server/java-home
            cp build.gradle.kts ../../../target/authenticator-server/libs/keyringd-authenticator-server.jar
            "#,
                    );
                    let build = AuthenticatorServerBuild::new(workspace.path()).with_java_home(&java_home);
                    let jar = build.prepare().unwrap();
                    assert_eq!(fs::read_to_string(&jar).unwrap(), "first inputs");
                    assert_eq!(
                        fs::read_to_string(
                            workspace
                                .path()
                                .join("target/authenticator-server/java-home")
                        )
                        .unwrap(),
                        java_home.to_str().unwrap()
                    );
                    fs::write(
                        workspace
                            .path()
                            .join("tests/keyringd-test-usbip/authenticator-server/build.gradle.kts"),
                        "changed inputs",
                    )
                    .unwrap();
                    assert_eq!(build.prepare().unwrap(), jar);
                    assert_eq!(fs::read_to_string(jar).unwrap(), "changed inputs");
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "authenticator server build" {
            case "Reports gradle failure and missing artifact" {
                verify "reports gradle failure and missing artifact" {
                    let workspace = tempfile::tempdir().unwrap();
                    project(workspace.path(), "exit 23");
                    let build = AuthenticatorServerBuild::new(workspace.path());
                    assert!(
                        build
                            .prepare()
                            .unwrap_err()
                            .to_string()
                            .contains("build failed")
                    );
                    project(workspace.path(), "exit 0");
                    assert!(
                        build
                            .prepare()
                            .unwrap_err()
                            .to_string()
                            .contains("JAR was not produced")
                    );
                }
            }
        }
    }
}
