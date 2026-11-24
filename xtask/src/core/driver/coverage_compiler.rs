use anyhow::{Context, Result};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Command,
};

/// Implements Cargo's RUSTC_WORKSPACE_WRAPPER protocol without a shell.
pub(crate) fn command(arguments: &[OsString], active: bool) -> Result<Option<Command>> {
    let Some(compiler) = arguments.first() else {
        return Ok(None);
    };
    if !active || Path::new(compiler).file_stem() != Some(OsStr::new("rustc")) {
        return Ok(None);
    }
    let arguments = &arguments[1..];
    let mut command = Command::new(compiler);
    // Keep response files on the command line to stay below Windows' length
    // limit, but inspect their UTF-8, newline-separated arguments as rustc does.
    command.args(arguments);
    let mut expanded = Vec::new();
    for argument in arguments {
        if let Some(path) = argument.to_str().and_then(|arg| arg.strip_prefix('@')) {
            expanded.extend(
                std::fs::read_to_string(path)
                    .with_context(|| format!("reading rustc response file {path}"))?
                    .lines()
                    .map(OsString::from),
            );
        } else {
            expanded.push(argument.clone());
        }
    }
    let arguments = &expanded;
    let build_tool = arguments.iter().any(|arg| arg == "proc-macro")
        || arguments
            .windows(2)
            .any(|pair| pair[0] == "--crate-name" && pair[1] == "build_script_build");
    if !build_tool && arguments.iter().any(|arg| arg == "--crate-name") {
        command.args([
            "-C",
            "instrument-coverage",
            // Windows' Rust LLVM tools may be built without zlib. WSL profiles
            // and their binary coverage maps must remain readable on that host.
            "-C",
            "llvm-args=-enable-name-compression=false",
        ]);
        let target = arguments
            .windows(2)
            .find(|pair| pair[0] == "--target")
            .map(|pair| pair[1].to_string_lossy());
        let darwin = target
            .map(|target| target.contains("-apple-darwin"))
            .unwrap_or(cfg!(target_os = "macos"));
        if darwin {
            // Darwin maps the counter sections in place; it does not use the
            // ELF/COFF runtime bias. Match Clang's profiling section alignment.
            for section in ["__llvm_prf_cnts", "__llvm_prf_bits", "__llvm_prf_data"] {
                command
                    .arg("-C")
                    .arg(format!("link-arg=-Wl,-sectalign,__DATA,{section},0x4000"));
            }
        } else {
            command.args(["-C", "llvm-args=-runtime-counter-relocation"]);
        }
    }
    Ok(Some(command))
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "command" {
            case "Darwin continuous profiles use page aligned sections instead of counter relocation" {
                verify "the target platform selects its required profiling mechanism" {
                    for (target, darwin) in [("aarch64-apple-darwin", true), ("x86_64-unknown-linux-musl", false), ("x86_64-pc-windows-msvc", false)] {
                        let args = ["rustc", "--crate-name", "probe", "--target", target].map(OsString::from);
                        let rustc = command(&args, true).unwrap().unwrap();
                        let flags = rustc.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>();
                        assert!(flags.iter().any(|flag| flag == "llvm-args=-enable-name-compression=false"));
                        assert_eq!(flags.iter().any(|flag| flag == "llvm-args=-runtime-counter-relocation"), !darwin);
                        for section in ["__llvm_prf_cnts", "__llvm_prf_bits", "__llvm_prf_data"] {
                            assert_eq!(flags.iter().any(|flag| flag == &format!("link-arg=-Wl,-sectalign,__DATA,{section},0x4000")), darwin);
                        }
                    }
                }
            }
            case "Instruments workspace crates while forwarding compiler probes and build tooling" {
                verify "only Rust source compilations receive profiling flags" {
                    let args = ["/toolchain/rustc", "--crate-name", "keyringd_core", "src/lib.rs"].map(OsString::from);
                    assert!(command(&args, false).unwrap().is_none());
                    let rustc = command(&args, true).unwrap().unwrap();
                    assert!(rustc.get_args().any(|arg| arg == "instrument-coverage"));
                    assert_eq!(rustc.get_program(), args[0]);
                    for level in ["debuginfo=0", "debuginfo=line-tables-only", "debuginfo=2"] {
                        let args = ["rustc", "--crate-name", "keyringd_core", "-C", level].map(OsString::from);
                        let rustc = command(&args, true).unwrap().unwrap();
                        let debug_flags = rustc.get_args().filter(|arg| arg.to_string_lossy().starts_with("debuginfo=")).collect::<Vec<_>>();
                        assert_eq!(debug_flags, [std::ffi::OsStr::new(level)]);
                    }
                    for args in [vec!["rustc", "-vV"], vec!["rustc", "--crate-name", "build_script_build"], vec!["rustc", "--crate-name", "macros", "--crate-type", "proc-macro"]] {
                        let args = args.into_iter().map(OsString::from).collect::<Vec<_>>();
                        let rustc = command(&args, true).unwrap().unwrap();
                        assert_eq!(rustc.get_args().collect::<Vec<_>>(), args[1..].iter().collect::<Vec<_>>());
                    }
                    assert!(command(&["test-with-report".into()], true).unwrap().is_none());
                }
            }
        }
    }
}
