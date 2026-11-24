use crate::core::model::{CoverageReport, Workspace};
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) struct CoverageSession {
    root: PathBuf,
    output: PathBuf,
    build: PathBuf,
}

impl CoverageSession {
    pub(crate) fn new(workspace: &Workspace, label: &str) -> Result<Self> {
        let output = workspace.target().join("coverage").join(label);
        // A fresh session must never reuse profiles or binaries from another run.
        // Labels are supplied by xtask, not arbitrary user paths.
        ensure!(
            !label.is_empty()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte)),
            "invalid coverage label"
        );
        let build = workspace.target().join("coverage-build").join(label);
        for path in [&output, &build] {
            fs::create_dir_all(path)?;
            let resolved = path.canonicalize()?;
            ensure!(
                resolved.starts_with(workspace.target().canonicalize()?)
                    && resolved != workspace.target().canonicalize()?,
                "coverage directory escapes target"
            );
            fs::remove_dir_all(path)?;
            fs::create_dir_all(path)?;
        }
        fs::create_dir_all(output.join("raw"))?;
        Ok(Self {
            root: workspace.root().to_owned(),
            output,
            build,
        })
    }

    pub(crate) fn configure(&self, command: &mut Command) -> Result<()> {
        // The relocated counters update the mapped profile even if a fixture is
        // killed. Hostname and PID separate host/container/application processes.
        let profile = self.output.join("raw/coverage-%h-%p-%m-%c.profraw");
        command
            .current_dir(&self.root)
            // Only workspace compilations need instrumentation. sccache's
            // compiler detection cannot wrap an arbitrary workspace wrapper.
            .env_remove("RUSTC_WRAPPER")
            .env("RUSTC_WORKSPACE_WRAPPER", std::env::current_exe()?)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", &self.build)
            .env("LLVM_PROFILE_FILE", profile)
            .env("KEYRINGD_COVERAGE_ACTIVE", "1");
        let mut wsl = std::env::var("WSLENV").unwrap_or_default();
        if !wsl.is_empty() {
            wsl.push(':');
        }
        wsl.push_str("LLVM_PROFILE_FILE/p");
        command.env("WSLENV", wsl);
        // Fail before a lengthy system test when the matching LLVM tools are absent.
        self.tool("llvm-profdata")?;
        self.tool("llvm-cov")?;
        Ok(())
    }

    pub(crate) fn run(&self, arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
        let mut command = Command::new(std::env::current_exe()?);
        self.configure(&mut command)?;
        let status = command
            .args(arguments)
            .status()
            .context("starting coverage test execution")?;
        ensure!(status.success(), "tests failed with {status}");
        Ok(())
    }

    pub(crate) fn build(&self, arguments: &[OsString]) -> Result<()> {
        let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        self.configure(&mut command)?;
        let status = command.arg("build").args(arguments).status()?;
        ensure!(status.success(), "instrumented build failed with {status}");
        println!("Instrumented package inputs: {}", self.build.display());
        Ok(())
    }

    pub(crate) fn export(&self, revision: &str, suite: &str, os: &str) -> Result<()> {
        let mut profiles = Vec::new();
        collect(&self.output.join("raw"), &mut profiles, &|path| {
            path.extension()
                .is_some_and(|extension| extension == "profraw")
        })?;
        ensure!(
            !profiles.is_empty(),
            "no raw coverage profiles were produced"
        );
        let profile_arguments = self.output.join("profiles.rsp");
        fs::write(&profile_arguments, response_arguments(&profiles))?;
        let indexed = self.output.join("coverage.profdata");
        let status = Command::new(self.tool("llvm-profdata")?)
            .arg("merge")
            .arg("--sparse")
            .arg(format!("@{}", profile_arguments.display()))
            .arg("-o")
            .arg(&indexed)
            .status()?;
        ensure!(status.success(), "merging raw coverage profiles failed");

        let mut objects = Vec::new();
        collect(&self.build, &mut objects, &is_coverage_binary)?;
        // Packaged WSL applications were compiled on other jobs. Their original
        // binaries carry the coverage mapping and must accompany the profiles.
        let imported = self.root.join("target/coverage-inputs");
        if imported.is_dir() {
            collect(&imported, &mut objects, &is_coverage_binary)?;
        }
        objects.sort();
        objects.dedup();
        ensure!(
            !objects.is_empty(),
            "no product coverage binaries were found"
        );
        let object_arguments = self.output.join("objects.rsp");
        let arguments = objects
            .iter()
            .map(|path| format!("-object={}", path.to_string_lossy().replace('\\', "/")))
            .collect::<Vec<_>>();
        fs::write(
            &object_arguments,
            arguments
                .iter()
                .map(|arg| quote_response(arg))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
        let raw_lcov = self.output.join("export.lcov");
        let status = Command::new(self.tool("llvm-cov")?)
            .args(["export", "-format=lcov"])
            .arg(format!("-instr-profile={}", indexed.display()))
            .arg(format!("@{}", object_arguments.display()))
            .stdout(Stdio::from(File::create(&raw_lcov)?))
            .status()?;
        ensure!(status.success(), "exporting product coverage failed");
        let report = CoverageReport::parse(&fs::read_to_string(raw_lcov)?, &self.root)?;
        if suite != "ut" {
            for package in match os {
                "linux" => &["keyringd-linux"][..],
                "windows" => &["keyringd-windows"][..],
                "macos" => &["keyringd-macos"][..],
                "wsl" => &["keyringd-wsl", "keyringd-windows"][..],
                _ => &[][..],
            } {
                report.require_executed_package(package)?;
            }
        }
        report.write_artifact(&self.root, &self.output, revision, suite, os)?;
        println!("Coverage artifacts: {}", self.output.display());
        Ok(())
    }

    fn tool(&self, name: &str) -> Result<PathBuf> {
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let root = Command::new(&rustc).args(["--print", "sysroot"]).output()?;
        ensure!(root.status.success(), "rustc --print sysroot failed");
        let version = Command::new(rustc).arg("-vV").output()?;
        let version = String::from_utf8(version.stdout)?;
        let host = version
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .context("missing Rust host triple")?;
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        let path = Path::new(String::from_utf8(root.stdout)?.trim())
            .join("lib/rustlib")
            .join(host)
            .join("bin")
            .join(format!("{name}{suffix}"));
        ensure!(
            path.is_file(),
            "{name} is missing; run `rustup component add llvm-tools-preview`"
        );
        Ok(path)
    }
}

fn collect(
    directory: &Path,
    output: &mut Vec<PathBuf>,
    matches: &impl Fn(&Path) -> bool,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        // Cargo's build directory also contains test executables. Traverse it,
        // but never follow directory symlinks or scan incremental metadata.
        if entry.file_type()?.is_dir()
            && !matches!(
                entry.file_name().to_str(),
                Some("incremental" | ".fingerprint")
            )
        {
            collect(&path, output, matches)?;
        } else if entry.file_type()?.is_file() && matches(&path) {
            output.push(path);
        }
    }
    Ok(())
}

fn is_coverage_binary(path: &Path) -> bool {
    // External specification/IT binaries may have arbitrary target names. Their
    // product code must be counted too; source filtering removes runner code.
    // Procedural macro libraries execute inside rustc and are build tooling.
    if path.file_stem().is_some_and(|name| {
        let name = name.to_string_lossy();
        name == "build-script-build"
            || name == "build_script_build"
            || name.starts_with("build_script_build-")
    }) || path
        .extension()
        .is_some_and(|extension| matches!(extension.to_str(), Some("dll" | "so" | "dylib")))
    {
        return false;
    }
    let mut magic = [0u8; 4];
    if File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_err()
    {
        return false;
    }
    magic.starts_with(b"MZ")
        || magic == *b"\x7fELF"
        || matches!(
            magic,
            [0xcf, 0xfa, 0xed, 0xfe]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xca, 0xfe, 0xba, 0xbe]
        )
}

fn quote_response(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "/").replace('"', "\\\""))
}

fn response_arguments(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| quote_response(&path.to_string_lossy()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "response_arguments" {
            case "Quotes spaces and normalizes Windows paths for LLVM response files" {
                verify "LLVM receives each path as a single argument" {
                    assert_eq!(response_arguments(&[PathBuf::from("C:\\work tree\\a.profraw"), PathBuf::from("/tmp/b.profraw")]), "\"C:/work tree/a.profraw\"\n\"/tmp/b.profraw\"");
                }
            }
        }
    }
}
