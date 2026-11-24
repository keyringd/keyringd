use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path};

/// Normalized product coverage for portable artifacts and source validation.
#[derive(Default)]
pub(crate) struct CoverageReport {
    files: BTreeMap<String, FileCoverage>,
}

#[derive(Default)]
struct FileCoverage {
    lines: BTreeMap<u32, u64>,
    functions: BTreeMap<(u32, String), u64>,
}

impl CoverageReport {
    pub(crate) fn parse(lcov: &str, workspace: &Path) -> Result<Self> {
        let packages = fs::read_dir(workspace)?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("Cargo.toml").is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("keyringd-"))
            .collect::<Vec<_>>();
        let mut report = Self::default();
        let mut file = None;
        let mut starts = BTreeMap::new();
        for line in lcov.lines() {
            if let Some(source) = line.strip_prefix("SF:") {
                file = Self::product_source(source, &packages);
                starts.clear();
            } else if line == "end_of_record" {
                file = None;
            } else if let Some(file) = file.as_ref() {
                let coverage = report.files.entry(file.clone()).or_default();
                if let Some(value) = line.strip_prefix("DA:") {
                    let mut fields = value.split(',');
                    let line = fields.next().context("missing coverage line")?.parse()?;
                    let count = fields
                        .next()
                        .context("missing coverage count")?
                        .parse::<u64>()?;
                    *coverage.lines.entry(line).or_default() += count;
                } else if let Some(value) = line.strip_prefix("FN:") {
                    let (line, name) = value.split_once(',').context("invalid FN record")?;
                    starts.insert(name.to_owned(), line.parse::<u32>()?);
                } else if let Some(value) = line.strip_prefix("FNDA:") {
                    let (count, name) = value.split_once(',').context("invalid FNDA record")?;
                    let start = *starts
                        .get(name)
                        .context("FNDA without a function location")?;
                    let location = format!(" (line {start})");
                    let name = name.strip_suffix(&location).unwrap_or(name);
                    let name = format!("{:#}", rustc_demangle::demangle(name));
                    *coverage.functions.entry((start, name)).or_default() +=
                        count.parse::<u64>()?;
                }
            }
        }
        report.files.retain(|_, file| !file.lines.is_empty());
        ensure!(
            !report.files.is_empty(),
            "no product coverage was collected"
        );
        Ok(report)
    }

    fn product_source(source: &str, packages: &[String]) -> Option<String> {
        let source = source.replace('\\', "/");
        let parts = source.split('/').collect::<Vec<_>>();
        let start = parts
            .iter()
            .position(|part| packages.iter().any(|name| name == part))?;
        let relative = parts[start..].join("/");
        if !parts.get(start + 1).is_some_and(|part| *part == "src")
            || parts[start..]
                .iter()
                .any(|part| matches!(*part, "tests" | "test_support" | "ui_test_support" | ".."))
            || relative.ends_with("/ui_test_app.rs")
        {
            return None;
        }
        Some(relative)
    }

    pub(crate) fn require_executed_package(&self, package: &str) -> Result<()> {
        ensure!(
            self.files.iter().any(|(name, file)| {
                name.starts_with(&format!("{package}/"))
                    && file.lines.values().any(|count| *count > 0)
            }),
            "no execution coverage was collected for {package}; check instrumentation and the application's LLVM_PROFILE_FILE environment"
        );
        Ok(())
    }

    pub(crate) fn lcov(&self) -> String {
        let mut output = String::new();
        for (name, file) in &self.files {
            writeln!(output, "SF:{name}").unwrap();
            // Demangled closures can have the same name at different locations.
            // LCOV identifies functions by name, so retain the source line in it.
            for (line, function) in file.functions.keys() {
                writeln!(output, "FN:{line},{function} (line {line})").unwrap();
            }
            for ((line, function), count) in &file.functions {
                writeln!(output, "FNDA:{count},{function} (line {line})").unwrap();
            }
            for (line, count) in &file.lines {
                writeln!(output, "DA:{line},{count}").unwrap();
            }
            writeln!(
                output,
                "FNF:{}\nFNH:{}\nLF:{}\nLH:{}",
                file.functions.len(),
                file.functions.values().filter(|count| **count > 0).count(),
                file.lines.len(),
                file.lines.values().filter(|count| **count > 0).count()
            )
            .unwrap();
            writeln!(output, "end_of_record").unwrap();
        }
        output
    }

    pub(crate) fn write_artifact(
        &self,
        root: &Path,
        output: &Path,
        revision: &str,
        suite: &str,
        os: &str,
    ) -> Result<()> {
        fs::create_dir_all(output)?;
        fs::write(output.join("lcov.info"), self.lcov())?;
        let hashes = self
            .files
            .keys()
            .map(|name| Ok((name.clone(), Self::source_hash(&root.join(name))?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        fs::write(
            output.join("metadata.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "revision": revision, "suite": suite, "os": os, "sources": hashes,
            }))?,
        )?;
        Ok(())
    }

    pub(crate) fn read_artifact(
        root: &Path,
        input: &Path,
        revision: &str,
    ) -> Result<(Self, String, String)> {
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(input.join("metadata.json"))?)?;
        ensure!(
            metadata["revision"].as_str() == Some(revision),
            "coverage revision mismatch: {}",
            input.display()
        );
        for (name, hash) in metadata["sources"]
            .as_object()
            .context("missing coverage source hashes")?
        {
            ensure!(
                Path::new(name)
                    .components()
                    .all(|part| matches!(part, Component::Normal(_))),
                "invalid coverage source path"
            );
            ensure!(
                hash.as_str() == Some(&Self::source_hash(&root.join(name))?),
                "coverage source differs from checkout: {name}"
            );
        }
        let data = Self::parse(&fs::read_to_string(input.join("lcov.info"))?, root)?;
        let suite = metadata["suite"]
            .as_str()
            .context("missing coverage suite")?
            .to_owned();
        let os = metadata["os"]
            .as_str()
            .context("missing coverage OS")?
            .to_owned();
        for label in [&suite, &os] {
            if label.is_empty()
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            {
                bail!("invalid coverage label: {label}");
            }
        }
        Ok((data, suite, os))
    }

    fn source_hash(path: &Path) -> Result<String> {
        // Normalize checkouts with different Git line-ending settings.
        let source =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(Sha256::digest(source.replace("\r\n", "\n").as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CoverageReport::require_executed_package" {
            case "Rejects system coverage without executed application code" {
                verify "test driver coverage cannot stand in for product execution" {
                    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
                    for count in [0, 1] {
                        let report = CoverageReport::parse(&format!("SF:/build/keyringd-core/src/lib.rs\nDA:1,{count}\nend_of_record"), root).unwrap();
                        assert_eq!(report.require_executed_package("keyringd-core").is_ok(), count > 0);
                        assert!(report.require_executed_package("keyringd-windows").is_err());
                    }
                }
            }
        }
        group "CoverageReport::read_artifact" {
            case "Rejects another revision or source content before aggregation" {
                verify "coverage only describes the checked out product sources" {
                    let directory = tempfile::tempdir().unwrap();
                    let root = directory.path();
                    fs::create_dir_all(root.join("keyringd-example/src")).unwrap();
                    fs::write(root.join("keyringd-example/Cargo.toml"), "").unwrap();
                    let source = root.join("keyringd-example/src/lib.rs");
                    fs::write(&source, "fn example() {}\n").unwrap();
                    let report = CoverageReport::parse("SF:/build/keyringd-example/src/lib.rs\nDA:1,1\nend_of_record", root).unwrap();
                    let output = root.join("artifact");
                    report.write_artifact(root, &output, "first", "ut", "linux").unwrap();
                    assert!(CoverageReport::read_artifact(root, &output, "first").is_ok());
                    assert!(CoverageReport::read_artifact(root, &output, "other").is_err());
                    fs::write(&source, "fn changed() {}\n").unwrap();
                    assert!(CoverageReport::read_artifact(root, &output, "first").is_err());
                }
            }
        }
        group "CoverageReport::parse" {
            case "Normalizes OS paths and Rust symbols and excludes test support" {
                verify "only product sources contribute to coverage" {
                    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
                    let data = CoverageReport::parse("SF:C:\\work\\keyringd-core\\src\\lib.rs\nDA:1,2\nend_of_record\nSF:/work/tests/runner/src/lib.rs\nDA:1,1\nend_of_record\nSF:/work/keyringd-core/src/test_support/foo.rs\nDA:1,1\nend_of_record\n", root).unwrap();
                    assert_eq!(data.files.len(), 1);
                    assert_eq!(data.files["keyringd-core/src/lib.rs"].lines[&1], 2);
                    assert!(CoverageReport::parse("SF:/work/xtask/src/main.rs\nDA:1,1\n", root).is_err());
                    let symbols = "SF:/build/keyringd-core/src/lib.rs\nFN:1,_ZN3foo17h0000000000000000E\nFN:1,_ZN3foo17h1111111111111111E\nFN:2,_ZN3foo17h2222222222222222E\nFNDA:1,_ZN3foo17h0000000000000000E\nFNDA:1,_ZN3foo17h1111111111111111E\nFNDA:0,_ZN3foo17h2222222222222222E\nDA:1,2\nDA:2,0\nend_of_record";
                    let normalized = CoverageReport::parse(symbols, root).unwrap();
                    let functions = &normalized.files["keyringd-core/src/lib.rs"].functions;
                    assert_eq!(functions.len(), 2);
                    assert_eq!(functions[&(1, "foo".to_owned())], 2);
                    assert_eq!(functions[&(2, "foo".to_owned())], 0);
                    assert_eq!(CoverageReport::parse(&normalized.lcov(), root).unwrap().lcov(), normalized.lcov());
                }
            }
        }
    }
}
