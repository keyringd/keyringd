use super::Environment;
use anyhow::{Context, Result, ensure};
use std::io::Write;
use std::path::Path;

const TEST_TCTI: &str = "swtpm:host=127.0.0.1,port=2321";
const TEST_ROOT: &str = "/var/lib/keyringd-e2e";

/// Only the runner-owned simulator is reset, after its application has stopped.
pub(super) async fn reset_software_tpm(
    environment: &Environment,
    tcti: &str,
    root: &Path,
) -> Result<()> {
    validate_target(
        std::env::var("KEYRINGD_E2E_SOFTWARE_TPM_RESET")
            .ok()
            .as_deref(),
        tcti,
        root,
    )?;
    let handles_before = command(
        environment,
        "tpm2_getcap",
        &["-T", tcti, "handles-transient"],
    )
    .await?;
    let properties_before = command(
        environment,
        "tpm2_getcap",
        &["-T", tcti, "properties-variable"],
    )
    .await?;
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(environment.artifact("software-tpm-reset.log"))?;
    writeln!(
        log,
        "reset at {:?}; before transient count={}; DA counter={:?}\nhandles:\n{}\nproperties:\n{}",
        std::time::SystemTime::now(),
        transient_count(&handles_before),
        lockout_counter(&properties_before),
        handles_before,
        properties_before
    )?;
    command(
        environment,
        "tpm2_flushcontext",
        &["-T", tcti, "--transient-object"],
    )
    .await?;
    command(
        environment,
        "tpm2_dictionarylockout",
        &["-T", tcti, "--clear-lockout"],
    )
    .await?;
    let handles_after = command(
        environment,
        "tpm2_getcap",
        &["-T", tcti, "handles-transient"],
    )
    .await?;
    let properties_after = command(
        environment,
        "tpm2_getcap",
        &["-T", tcti, "properties-variable"],
    )
    .await?;
    writeln!(
        log,
        "after transient count={}; DA counter={:?}\nhandles:\n{}\nproperties:\n{}",
        transient_count(&handles_after),
        lockout_counter(&properties_after),
        handles_after,
        properties_after
    )?;
    ensure!(
        transient_count(&handles_after) == 0,
        "software TPM still has transient objects after reset"
    );
    ensure!(
        lockout_counter(&properties_after) == Some(0),
        "software TPM DA counter was not cleared"
    );
    Ok(())
}

fn validate_target(flag: Option<&str>, tcti: &str, root: &Path) -> Result<()> {
    ensure!(
        flag == Some("1"),
        "software TPM reset requires the dedicated testbed flag"
    );
    ensure!(
        root == Path::new(TEST_ROOT),
        "software TPM reset requires the dedicated testbed root"
    );
    ensure!(
        tcti == TEST_TCTI,
        "refusing to reset an unknown TPM TCTI: {tcti}"
    );
    Ok(())
}
async fn command(environment: &Environment, program: &str, arguments: &[&str]) -> Result<String> {
    let output = environment
        .run(program, arguments)
        .await
        .with_context(|| format!("could not run dedicated software TPM reset command {program}"))?;
    ensure!(
        output.status.success(),
        "{program} failed: {}{}",
        output.stdout,
        output.stderr
    );
    Ok(output.stdout)
}
fn transient_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_start().starts_with("- 0x"))
        .count()
}
fn lockout_counter(properties: &str) -> Option<u32> {
    let raw = properties
        .lines()
        .find_map(|line| line.strip_prefix("TPM2_PT_LOCKOUT_COUNTER: "))?;
    u32::from_str_radix(raw.strip_prefix("0x")?, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "validate_target" {
            case "Only the explicit runner simulator can be reset" {
                verify "allow TPM reset only for the explicitly configured test simulator" {
                    let root = Path::new(TEST_ROOT);
                    assert!(validate_target(Some("1"), TEST_TCTI, root).is_ok());
                    for tcti in [
                        "device:/dev/tpmrm0",
                        "device:/dev/tpm0",
                        "device",
                        "tabrmd",
                        "swtpm:host=other,port=2321",
                        "swtpm:host=127.0.0.1,port=2322",
                    ] {
                        assert!(validate_target(Some("1"), tcti, root).is_err());
                    }
                    assert!(validate_target(None, TEST_TCTI, root).is_err());
                    assert!(validate_target(Some("0"), TEST_TCTI, root).is_err());
                    assert!(validate_target(Some("1"), TEST_TCTI, Path::new("/home/user")).is_err());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "transient_count" {
            case "Parses tpm2 tools capability output" {
                verify "recognize the expected capability values in tpm2-tools output" {
                    assert_eq!(transient_count("- 0x80000000\n- 0x80000001\n"), 2);
                    assert_eq!(transient_count(""), 0);
                    let properties =
                        "TPM2_PT_HR_LOADED: 0x2\nTPM2_PT_LOCKOUT_COUNTER: 0x3\nTPM2_PT_MAX_AUTH_FAIL: 0x3\n";
                    assert_eq!(lockout_counter(properties), Some(3));
                    assert_eq!(lockout_counter("TPM2_PT_LOCKOUT_COUNTER: 0x0\n"), Some(0));
                    assert_eq!(lockout_counter("unrecognized output"), None);
                }
            }
        }
    }
}
