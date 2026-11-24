use std::backtrace::Backtrace;
use std::sync::Once;

/// Records failures at execution boundaries through the application logger.
pub struct FailureDiagnostics;

impl FailureDiagnostics {
    pub fn install() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let thread = std::thread::current();
                tracing::error!(
                    panic = %info,
                    pid = std::process::id(),
                    thread = ?thread.id(),
                    name = ?thread.name(),
                    backtrace = %Backtrace::force_capture(),
                    "Unhandled Rust panic",
                );
                previous(info);
            }));
        });
    }

    /// Logs the error's own diagnostic representation, including any backtrace
    /// it already carries, plus the current execution-boundary stack. Returning
    /// a Result does not preserve the originating call stack on its own.
    pub fn record_error(operation: &str, error: &dyn std::fmt::Debug) {
        tracing::error!(
            operation,
            ?error,
            thread = ?std::thread::current().id(),
            boundary_backtrace = %Backtrace::force_capture(),
            "Unhandled error",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CHILD: &str = "KEYRINGD_PANIC_DIAGNOSTICS_TEST";

    // An ignored child-process entry point, exercised by the reported parent scenario.
    #[test]
    #[ignore = "invoked only by the FailureDiagnostics scenario"]
    fn diagnostics_child_process() {
        let mode = std::env::var(CHILD).unwrap();

        let path = std::env::var_os("KEYRINGD_PANIC_DIAGNOSTICS_LOG").unwrap();
        let file = std::fs::File::create(path).unwrap();
        let previous = std::panic::take_hook();
        let marker_file = file.try_clone().unwrap();
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::sync::Mutex::new(file))
            .init();
        std::panic::set_hook(Box::new(move |info| {
            let _ = writeln!(&marker_file, "previous panic hook");
            previous(info);
        }));
        FailureDiagnostics::install();
        FailureDiagnostics::install();
        match mode.as_str() {
            "main" => panic!("main panic marker"),
            "error" => FailureDiagnostics::record_error(
                "test error boundary",
                &std::io::Error::other("result error marker"),
            ),
            "task" => {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name("diagnostics-worker")
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    assert!(
                        tokio::spawn(async { panic!("task panic marker") })
                            .await
                            .unwrap_err()
                            .is_panic()
                    );
                });
            }
            _ => unreachable!(),
        }
    }

    keyringd_testkit::scenario! {
        group "FailureDiagnostics::install" {
            case "Records panics and returned errors without rust backtrace" {
                verify "record the failure cause and omit Rust backtrace frames" {


                    let directory = tempfile::tempdir().unwrap();
                    for mode in ["main", "task", "error"] {
                        let path = directory.path().join(format!("{mode}.log"));
                        let status = std::process::Command::new(std::env::current_exe().unwrap())
                                    .args([
                                        "--exact",
                                        "util::failure_diagnostics::tests::diagnostics_child_process",
                                        "--ignored",
                                        "--nocapture",
                                    ])
                                    .env(CHILD, mode)
                                    .env("KEYRINGD_PANIC_DIAGNOSTICS_LOG", &path)
                                    .env("RUST_LOG", "error")
                                    .env("RUST_BACKTRACE", "0")
                                    .output()
                                    .unwrap();
                        assert_eq!(status.status.success(), mode != "main", "child {mode}: {}", String::from_utf8_lossy(&status.stderr));
                        let log = std::fs::read_to_string(path).unwrap();
                        if mode == "error" {
                            assert!(log.contains("result error marker"), "{log}");
                            assert!(log.contains("boundary_backtrace="), "{log}");
                        } else {
                            assert!(log.contains(&format!("{mode} panic marker")), "{log}");
                            assert!(log.contains("failure_diagnostics.rs:"), "{log}");
                            assert!(log.contains("previous panic hook"), "{log}");
                            assert_eq!(log.matches("previous panic hook").count(), 1, "{log}");
                        }
                        assert!(log.contains("   0:"), "{log}");
                        if mode == "task" {
                            assert!(log.contains("diagnostics-worker"), "{log}");
                        }
                    }
                }
            }
        }
    }
}
