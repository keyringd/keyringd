use anyhow::{Context, Result, ensure};
use plist::Value;
pub(crate) const BUNDLE_ID: &str = "com.keyringd.Keyringd";

pub(crate) fn set_bundle_version(info: &mut Value, version: &semver::Version) -> Result<()> {
    let info = info
        .as_dictionary_mut()
        .context("Info.plist must be a dictionary")?;
    ensure!(
        info.get("CFBundleIdentifier").and_then(Value::as_string) == Some(BUNDLE_ID),
        "unexpected bundle identifier"
    );
    ensure!(
        info.get("CFBundleExecutable").and_then(Value::as_string) == Some("keyringd"),
        "unexpected bundle executable"
    );
    info.insert(
        "CFBundleShortVersionString".into(),
        format!("{}.{}.{}", version.major, version.minor, version.patch).into(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "set_bundle_version" {
            case "Bundle metadata uses the product version without semver suffixes" {
                verify "write the base product version into the app bundle metadata" {
                    let mut info =
                        Value::from_reader_xml(include_bytes!("../../../../packaging/macos/Info.plist").as_slice())
                            .unwrap();
                    set_bundle_version(
                        &mut info,
                        &semver::Version::parse("1.2.3-nightly.4+abcd").unwrap(),
                    )
                    .unwrap();
                    assert_eq!(
                        info.as_dictionary().unwrap()["CFBundleShortVersionString"].as_string(),
                        Some("1.2.3")
                    );
                    info.as_dictionary_mut()
                        .unwrap()
                        .insert("CFBundleIdentifier".into(), "wrong.bundle".into());
                    assert!(set_bundle_version(&mut info, &semver::Version::parse("1.2.3").unwrap()).is_err());
                }
            }
        }
    }
}
