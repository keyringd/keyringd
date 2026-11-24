use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use keyringd_core::test_support::TestFidoMode;

pub(super) struct FixtureArguments {
    pub(super) control_file: PathBuf,
    pub(super) state_file: PathBuf,
    pub(super) authenticator_usb: Option<SocketAddr>,
    pub(super) test_fido_mode: Option<TestFidoMode>,
    pub(super) accept_remote_hybrid_advertisements: bool,
}

impl FixtureArguments {
    pub(super) fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self> {
        ensure!(
            arguments.next().as_deref() == Some("--control-file"),
            "--control-file is required"
        );
        let control_file = arguments
            .next()
            .context("missing control file path")?
            .into();
        ensure!(
            arguments.next().as_deref() == Some("--state-file"),
            "--state-file is required"
        );
        let state_file = arguments.next().context("missing state file path")?.into();
        let (authenticator_usb, test_fido_mode) = match arguments.next().as_deref() {
            Some("--authenticator-usb") => (
                Some(
                    arguments
                        .next()
                        .context("missing authenticator USB/IP address")?
                        .parse()
                        .context("invalid authenticator USB/IP address")?,
                ),
                None,
            ),
            Some("--test-fido-mode") => (
                None,
                Some(serde_json::from_value(serde_json::Value::String(
                    arguments.next().context("missing test FIDO mode")?,
                ))?),
            ),
            _ => anyhow::bail!("--authenticator-usb or --test-fido-mode is required"),
        };
        let accept_remote_hybrid_advertisements = match arguments.next().as_deref() {
            Some("--accept-remote-hybrid-advertisements") => true,
            None => false,
            _ => anyhow::bail!("unexpected fixture argument"),
        };
        ensure!(arguments.next().is_none(), "unexpected fixture argument");
        ensure!(
            !accept_remote_hybrid_advertisements || authenticator_usb.is_some(),
            "remote Hybrid advertisements require the native FIDO backend"
        );
        Ok(Self {
            control_file,
            state_file,
            authenticator_usb,
            test_fido_mode,
            accept_remote_hybrid_advertisements,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(backend: &[&str]) -> Result<FixtureArguments> {
        FixtureArguments::parse(
            ["--control-file", "control", "--state-file", "state"]
                .into_iter()
                .chain(backend.iter().copied())
                .map(str::to_owned),
        )
    }

    keyringd_testkit::scenario! {
        group "FixtureArguments::parse" {
            case "Advertisement relay requires explicit native option" {
                verify "enable remote advertisements only with the native backend and explicit option" {
                    let native = ["--authenticator-usb", "127.0.0.1:3240"];
                    assert!(!parse(&native).unwrap().accept_remote_hybrid_advertisements);
                    assert!(
                        parse(&[
                            "--authenticator-usb",
                            "127.0.0.1:3240",
                            "--accept-remote-hybrid-advertisements",
                        ])
                        .unwrap()
                        .accept_remote_hybrid_advertisements
                    );
                    assert!(
                        parse(&[
                            "--test-fido-mode",
                            "success",
                            "--accept-remote-hybrid-advertisements",
                        ])
                        .is_err()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "FixtureArguments::parse" {
            case "Backend is required and exclusive" {
                verify "reject missing or conflicting fixture backend arguments" {
                    assert!(parse(&[]).is_err());
                    let native = parse(&["--authenticator-usb", "127.0.0.1:3240"]).unwrap();
                    assert_eq!(native.control_file, PathBuf::from("control"));
                    assert_eq!(native.state_file, PathBuf::from("state"));
                    assert!(native.authenticator_usb.is_some());
                    assert!(native.test_fido_mode.is_none());
                    let mock = parse(&["--test-fido-mode", "bluetooth_disabled"]).unwrap();
                    assert_eq!(mock.test_fido_mode, Some(TestFidoMode::BluetoothDisabled));
                    assert!(parse(&["--test-fido-mode", "unknown"]).is_err());
                    assert!(
                        parse(&[
                            "--authenticator-usb",
                            "127.0.0.1:3240",
                            "--test-fido-mode",
                            "success"
                        ])
                        .is_err()
                    );
                }
            }
        }
    }
}
