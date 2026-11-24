use crate::core::driver::macos_provisioning;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::core::driver::MacosReleaseCredentials;
use crate::core::driver::MacosSigningKeychain;
use crate::core::driver::MacosTools;

pub(crate) fn create_notarized_archive(
    app: &Path,
    version: &semver::Version,
    output: &Path,
    tools: &mut MacosTools,
) -> Result<()> {
    let credentials = MacosReleaseCredentials::from_env(tools)?;
    let architecture = tools.text(
        Command::new("lipo")
            .arg("-archs")
            .arg(app.join("Contents/MacOS/keyringd")),
    )?;
    let destination = output.join(package_name(version, &architecture)?);
    let temporary = tempfile::tempdir()?;
    let directory = temporary.path();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    write_secret(
        &directory.join("profile.provisionprofile"),
        &decode(&credentials.profile)?,
    )?;
    let decoded = tools.checked(
        Command::new("security")
            .args(["cms", "-D", "-i"])
            .arg(directory.join("profile.provisionprofile")),
    )?;
    let profile = plist::Value::from_reader(std::io::Cursor::new(decoded.stdout))?;
    let openssl = PathBuf::from(tools.text(Command::new("brew").args(["--prefix", "openssl@3"]))?)
        .join("bin/openssl");
    let password = tools.text(Command::new(&openssl).args(["rand", "-hex", "32"]))?;
    ensure!(
        password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "invalid temporary keychain password"
    );
    tools.mask(&password);
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        println!("::add-mask::{password}");
    }
    let application_name = prepare_identity(
        directory,
        &credentials.certificate,
        &credentials.certificate_password,
        &password,
        &openssl,
        tools,
    )?;
    ensure!(
        application_name.starts_with("Developer ID Application: "),
        "expected a Developer ID Application certificate"
    );
    let fingerprint = tools.text(
        Command::new(&openssl)
            .args(["x509", "-in"])
            .arg(directory.join("certificate.pem"))
            .args(["-noout", "-fingerprint", "-sha1"]),
    )?;
    let identity = fingerprint
        .split_once('=')
        .context("missing signing certificate fingerprint")?
        .1
        .replace(':', "");
    ensure!(
        identity.len() == 40 && identity.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "invalid signing certificate fingerprint"
    );
    macos_provisioning::entitlements(
        &profile,
        &std::fs::read(directory.join("certificate.der"))?,
        &credentials.team,
        SystemTime::now(),
    )?
    .to_file_xml(directory.join("entitlements.plist"))?;
    std::fs::copy(
        directory.join("profile.provisionprofile"),
        app.join("Contents/embedded.provisionprofile"),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            app.join("Contents/embedded.provisionprofile"),
            std::fs::Permissions::from_mode(0o644),
        )?;
    }

    // Reimporting the same private key in one keychain retains the first tool's access list.
    println!("Importing macOS application signing key...");
    let mut app_keychain =
        MacosSigningKeychain::import(directory, &password, "/usr/bin/codesign", tools)?;
    let identities = tools.text(
        Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .arg(app_keychain.path()),
    )?;
    ensure!(
        identities.split_whitespace().any(|word| word == identity),
        "imported certificate is not a valid code-signing identity (check expiration and trust)"
    );
    println!("Signing macOS app bundle...");
    tools.checked(
        Command::new("codesign")
            .args(["--force", "--sign", &identity, "--keychain"])
            .arg(app_keychain.path())
            .args(["--options", "runtime", "--timestamp", "--entitlements"])
            .arg(directory.join("entitlements.plist"))
            .arg(app),
    )?;
    tools.checked(
        Command::new("codesign")
            .args(["--verify", "--deep", "--strict", "--verbose=2"])
            .arg(app),
    )?;
    let signature = tools.checked(
        Command::new("codesign")
            .args(["-dv", "--verbose=4"])
            .arg(app),
    )?;
    ensure!(
        String::from_utf8_lossy(&signature.stderr)
            .lines()
            .any(|line| line == format!("TeamIdentifier={}", credentials.team)),
        "signing certificate belongs to a different team"
    );
    app_keychain.cleanup()?;
    let submission = directory.join("submission.zip");
    println!("Creating macOS notarization archive...");
    tools.checked(
        Command::new("ditto")
            .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
            .arg(app)
            .arg(&submission),
    )?;
    let notary_command = |operation: &str| {
        let mut command = Command::new("xcrun");
        command.args(["notarytool", operation]);
        command
    };
    let authenticate = |command: &mut Command| {
        command.args([
            "--apple-id",
            &credentials.apple_id,
            "--team-id",
            &credentials.team,
            "--password",
            &credentials.apple_password,
        ]);
    };
    let mut submit = notary_command("submit");
    submit.arg(&submission);
    authenticate(&mut submit);
    submit.args(["--wait", "--timeout", "30m", "--output-format", "json"]);
    println!("Submitting macOS app for notarization...");
    let response = tools.output(&mut submit)?;
    // Keep the response even when notarization succeeds: subsequent stapler failures
    // need the submission ID for investigation. It contains no signing credentials.
    println!(
        "Notarization response:\n{}",
        tools.redact(&String::from_utf8_lossy(&response.stdout))
    );
    if let Err(error) = ensure_notarization_accepted(response.status.success(), &response.stdout) {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&response.stdout)
            && let Some(id) = value.get("id").and_then(serde_json::Value::as_str)
        {
            eprintln!("Notarization submission: {id}");
            let mut log = notary_command("log");
            log.arg(id);
            authenticate(&mut log);
            if let Ok(log) = tools.output(&mut log) {
                eprintln!("{}", tools.redact(&String::from_utf8_lossy(&log.stdout)));
            }
        }
        eprintln!(
            "{}",
            tools.redact(&String::from_utf8_lossy(&response.stderr))
        );
        return Err(error);
    }
    finalize(app, &destination, |command| {
        tools.checked(command).map(|_| ())
    })?;
    println!("{}", destination.display());
    Ok(())
}

fn prepare_identity(
    directory: &Path,
    encoded: &str,
    certificate_password: &str,
    password: &str,
    openssl: &Path,
    tools: &MacosTools,
) -> Result<String> {
    write_secret(&directory.join("certificate.p12"), &decode(encoded)?)?;
    let openssl_command = || {
        let mut command = Command::new(openssl);
        command
            .env("KEYRINGD_P12_PASSWORD", certificate_password)
            .env("KEYRINGD_IMPORT_PASSWORD", password);
        command
    };
    // Rewrap OpenSSL 3's PKCS#12 defaults into the encrypted encoding Keychain accepts.
    tools.checked(
        openssl_command()
            .args(["pkcs12", "-in"])
            .arg(directory.join("certificate.p12"))
            .args([
                "-passin",
                "env:KEYRINGD_P12_PASSWORD",
                "-passout",
                "env:KEYRINGD_IMPORT_PASSWORD",
                "-aes256",
                "-out",
            ])
            .arg(directory.join("identity.pem")),
    )?;
    tools.checked(
        openssl_command()
            .args(["pkcs12", "-export", "-in"])
            .arg(directory.join("identity.pem"))
            .args([
                "-passin",
                "env:KEYRINGD_IMPORT_PASSWORD",
                "-passout",
                "env:KEYRINGD_IMPORT_PASSWORD",
                "-keypbe",
                "PBE-SHA1-3DES",
                "-certpbe",
                "PBE-SHA1-3DES",
                "-macalg",
                "sha1",
                "-out",
            ])
            .arg(directory.join("import.p12")),
    )?;
    std::fs::remove_file(directory.join("identity.pem"))?;
    tools.checked(
        openssl_command()
            .args(["pkcs12", "-in"])
            .arg(directory.join("certificate.p12"))
            .args([
                "-passin",
                "env:KEYRINGD_P12_PASSWORD",
                "-clcerts",
                "-nokeys",
                "-out",
            ])
            .arg(directory.join("certificate.pem")),
    )?;
    std::fs::remove_file(directory.join("certificate.p12"))?;
    tools.checked(
        openssl_command()
            .args(["x509", "-in"])
            .arg(directory.join("certificate.pem"))
            .args(["-outform", "DER", "-out"])
            .arg(directory.join("certificate.der")),
    )?;
    let subject = tools.text(
        openssl_command()
            .args(["x509", "-in"])
            .arg(directory.join("certificate.pem"))
            .args(["-noout", "-subject", "-nameopt", "multiline"]),
    )?;
    subject
        .lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(name, value)| (name.trim() == "commonName").then(|| value.trim().to_owned()))
        .context("missing certificate common name")
}

fn package_name(version: &semver::Version, architecture: &str) -> Result<String> {
    ensure!(
        matches!(architecture, "arm64" | "x86_64"),
        "unsupported macOS binary architecture: {architecture}"
    );
    Ok(format!("keyringd-{version}-{architecture}.app.zip"))
}

fn decode(value: &str) -> Result<Vec<u8>> {
    STANDARD
        .decode(
            value
                .bytes()
                .filter(|byte| !byte.is_ascii_whitespace())
                .collect::<Vec<_>>(),
        )
        .context("invalid base64 release setting")
}

fn write_secret(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)?;
    Ok(())
}

fn ensure_notarization_accepted(success: bool, response: &[u8]) -> Result<()> {
    let response: serde_json::Value = serde_json::from_slice(response)
        .context("invalid notarization response; no new release archive produced")?;
    ensure!(
        success && response.get("status").and_then(serde_json::Value::as_str) == Some("Accepted"),
        "notarization did not complete with Accepted status; no new release archive produced"
    );
    Ok(())
}

fn finalize(
    app: &Path,
    destination: &Path,
    mut run: impl FnMut(&mut Command) -> Result<()>,
) -> Result<()> {
    run(Command::new("xcrun").args(["stapler", "staple"]).arg(app))
        .context("attach the notarization ticket (stapler staple)")?;
    run(Command::new("xcrun").args(["stapler", "validate"]).arg(app))
        .context("validate the notarization ticket (stapler validate)")?;
    run(Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app))
    .context("verify the application signature (codesign)")?;
    run(Command::new("spctl")
        .args(["--assess", "--type", "execute", "--verbose=2"])
        .arg(app))
    .context("assess the stapled application (spctl)")?;
    let staging = tempfile::tempdir_in(destination.parent().context("archive has no parent")?)?;
    let archive = staging.path().join("keyringd.app.zip");
    run(Command::new("ditto")
        .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
        .arg(app)
        .arg(&archive))
    .context("archive the stapled application (ditto)")?;
    ensure!(
        std::fs::metadata(&archive)?.len() > 0,
        "empty release archive"
    );
    std::fs::rename(archive, destination).context("cannot publish the verified release archive")?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ensure_notarization_accepted" {
            case "Notarization requires successful exit and accepted status" {
                verify "require both a zero exit status and an Accepted notarization response" {
                    assert!(
                        ensure_notarization_accepted(true, br#"{"status":"Accepted","id":"submission"}"#).is_ok()
                    );
                    for (success, response) in [
                        (false, r#"{"status":"Accepted"}"#),
                        (true, r#"{"status":"Invalid"}"#),
                        (true, r#"{"status":"In Progress"}"#),
                        (true, r#"{"id":"submission"}"#),
                        (true, "not JSON"),
                    ] {
                        assert!(ensure_notarization_accepted(success, response.as_bytes()).is_err());
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "package_name" {
            case "Package names preserve nightly versions and use binary architecture" {
                verify "form package names using the full nightly version and the binary architecture" {
                    assert_eq!(
                        package_name(&semver::Version::parse("1.2.3-nightly.4").unwrap(), "arm64").unwrap(),
                        "keyringd-1.2.3-nightly.4-arm64.app.zip"
                    );
                    assert_eq!(
                        package_name(&semver::Version::parse("1.2.3").unwrap(), "x86_64").unwrap(),
                        "keyringd-1.2.3-x86_64.app.zip"
                    );
                    assert!(package_name(&semver::Version::parse("1.2.3").unwrap(), "arm64 x86_64").is_err());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "finalize" {
            case "Every final validation failure preserves the existing package" {
                verify "leave the existing release archive in place when any final validation fails" {
                    let directory = tempfile::tempdir().unwrap();
                    let destination = directory.path().join("release.app.zip");
                    let package = directory.path().join("submission.app.zip");
                    for (fail_at, stage) in [
                        "stapler staple",
                        "stapler validate",
                        "codesign",
                        "spctl",
                        "ditto",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        std::fs::write(&destination, b"previous release").unwrap();
                        let mut calls = 0;
                        let result = finalize(&package, &destination, |_| {
                            let current = calls;
                            calls += 1;
                            ensure!(current != fail_at, "native tool failed");
                            Ok(())
                        });
                        let error = result.unwrap_err();
                        assert!(error.to_string().contains(stage));
                        assert!(format!("{error:#}").contains("native tool failed"));
                        assert_eq!(calls, fail_at + 1);
                        assert_eq!(std::fs::read(&destination).unwrap(), b"previous release");
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "decode" {
            case "Base64 secrets accept line wrapping but reject corruption" {
                verify "decode wrapped Base64 and reject an invalid secret value" {
                    assert_eq!(decode(" Y2Vy\r\ndGlmaWNhdGU=\n").unwrap(), b"certificate");
                    assert!(decode("not-base64!").is_err());
                }
            }
        }
    }
}
