use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// Collects executable artifacts from a Cargo JSON message stream.
#[derive(Default)]
pub struct CargoBuildArtifacts {
    executables: BTreeMap<String, BTreeSet<PathBuf>>,
}
impl CargoBuildArtifacts {
    /// Ingests one JSON line; returns a rendered compiler diagnostic for the caller to print.
    pub fn push_line(&mut self, line: &str) -> Result<Option<String>> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        let message: serde_json::Value =
            serde_json::from_str(line).context("invalid Cargo JSON message")?;
        if message["reason"] == "compiler-artifact"
            && let (Some(target), Some(executable)) = (
                message["target"]["name"].as_str(),
                message["executable"].as_str(),
            )
        {
            self.executables
                .entry(target.into())
                .or_default()
                .insert(executable.into());
        }
        Ok(message["message"]["rendered"].as_str().map(str::to_owned))
    }
    /// Requires exactly one executable for the target. Build success is checked by the caller.
    pub fn executable(&self, target: &str) -> Result<PathBuf> {
        let paths = self
            .executables
            .get(target)
            .with_context(|| format!("Cargo did not report an executable for {target}"))?;
        ensure!(
            paths.len() == 1,
            "Cargo reported multiple executables for {target}: {paths:?}"
        );
        Ok(paths.first().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "CargoBuildArtifacts::push_line" {
            case "Extracts the requested target and preserves diagnostics" {
                verify "select the requested Cargo executable and retain diagnostic messages" {
                    std::thread::spawn(|| {
                    let mut artifacts = CargoBuildArtifacts::default();
                            artifacts.push_line(r#"{"reason":"compiler-artifact","target":{"name":"dependency"},"executable":null}"#).unwrap();
                            artifacts.push_line(r#"{"reason":"compiler-artifact","target":{"name":"scenarios"},"executable":"/work with spaces/scenarios"}"#).unwrap();
                            assert_eq!(
                                artifacts.executable("scenarios").unwrap(),
                                PathBuf::from("/work with spaces/scenarios")
                            );
                            assert_eq!(
                                artifacts
                                    .push_line(r#"{"reason":"compiler-message","message":{"rendered":"warning\n"}}"#)
                                    .unwrap()
                                    .as_deref(),
                                Some("warning\n")
                            );
                            assert!(artifacts.executable("dependency").is_err());
                    }).join().unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CargoBuildArtifacts::push_line" {
            case "Rejects invalid json and ambiguous artifacts" {
                verify "reject malformed Cargo messages and multiple matching executable artifacts" {
                    std::thread::spawn(|| {
                    let mut artifacts = CargoBuildArtifacts::default();
                            assert!(artifacts.push_line("not json").is_err());
                            for executable in ["first", "second"] {
                                artifacts.push_line(&serde_json::json!({"reason":"compiler-artifact","target":{"name":"scenarios"},"executable":executable}).to_string()).unwrap();
                            }
                            assert!(artifacts.executable("scenarios").is_err());
                    }).join().unwrap();
                }
            }
        }
    }
}
