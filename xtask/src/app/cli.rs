use super::task::TaskCommand;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "cargo xtask", about = "keyringd repository tasks")]
struct Cli {
    /// Measures product coverage while running tests (requires llvm-tools-preview).
    #[arg(long, global = true)]
    coverage: bool,
    #[command(subcommand)]
    task: TaskCommand,
}

pub(crate) fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.coverage {
        cli.task.exec_with_coverage()
    } else {
        cli.task.exec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "Cli::try_parse_from" {
            case "Coverage is available to existing test commands without consuming Cargo arguments" {
                verify "coverage flags and forwarded test arguments remain distinct" {
                    for command in ["test-with-report", "host-tests-with-report", "linux-system-test", "linux-system-test-with-report", "windows-system-test", "macos-system-test", "wsl-system-test"] {
                        assert!(Cli::try_parse_from(["xtask", command, "--coverage"]).unwrap().coverage);
                        assert!(Cli::try_parse_from(["xtask", "--coverage", command]).unwrap().coverage);
                    }
                    let parsed = Cli::try_parse_from(["xtask", "test-with-report", "--coverage", "--", "-p", "keyringd-core"]).unwrap();
                    assert!(parsed.coverage);
                    let TaskCommand::TestWithReport(task) = parsed.task else { panic!("unexpected command") };
                    assert_eq!(task.cargo_test_arguments, ["-p", "keyringd-core"].map(std::ffi::OsString::from));
                }
            }
        }
    }
}
