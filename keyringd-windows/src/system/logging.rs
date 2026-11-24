pub(crate) fn init_windows(data_dir: &std::path::Path) -> std::io::Result<()> {
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, "keyringd.log");
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_env_filter(
            filter.add_directive(
                "keyringd_context=info"
                    .parse()
                    .expect("valid context log directive"),
            ),
        )
        .with_writer(appender.with_max_level(tracing::Level::TRACE))
        .try_init()
        .map_err(std::io::Error::other)?;
    keyringd_core::util::FailureDiagnostics::install();
    tracing::info!("keyringd started");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "init_windows" {
            case "Records failures in the existing application log" {
                verify "append failure details to the pre-existing application log" {
                    (|| {
                        const CHILD: &str = "KEYRINGD_LOGGING_FAILURE_TEST";
                        if let Ok(mode) = std::env::var(CHILD) {
                            let directory = std::env::var_os("KEYRINGD_LOGGING_TEST_DIRECTORY").unwrap();
                            init_windows(std::path::Path::new(&directory)).unwrap();
                            if mode == "panic" {
                                panic!("Windows panic diagnostic marker");
                            }
                            let error = std::thread::spawn(error_at_origin).join().unwrap();
                            keyringd_core::util::FailureDiagnostics::record_error("test boundary", &error);
                            return;
                        }
                        let directory = tempfile::tempdir().unwrap();
                        for mode in ["panic", "error"] {
                            let child_directory = directory.path().join(mode);
                            let output = std::process::Command::new(std::env::current_exe().unwrap())
                                .args([
                                    "--exact",
                                    "system::logging::tests::records_failures_in_the_existing_application_log",
                                    "--nocapture",
                                ])
                                .env(CHILD, mode)
                                .env("KEYRINGD_LOGGING_TEST_DIRECTORY", &child_directory)
                                .env("RUST_LOG", "error")
                                .env("RUST_BACKTRACE", "0")
                                .env(
                                    "RUST_LIB_BACKTRACE",
                                    if mode == "error" { "1" } else { "0" },
                                )
                                .output()
                                .unwrap();
                            assert_eq!(output.status.success(), mode == "error", "{output:?}");
                            let log: String = std::fs::read_dir(child_directory.join("logs"))
                                .unwrap()
                                .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
                                .collect();
                            if mode == "panic" {
                                assert!(log.contains("Windows panic diagnostic marker"), "{log}");
                                assert!(log.contains("logging.rs:"), "{log}");
                            } else {
                                assert!(log.contains("original error marker"), "{log}");
                                assert!(
                                    log.split("boundary_backtrace=")
                                        .next()
                                        .unwrap()
                                        .contains("error_at_origin"),
                                    "{log}"
                                );
                                assert!(log.contains("boundary_backtrace="), "{log}");
                            }
                            assert!(log.contains("   0:"), "{log}");
                        }
                    })();
                }
            }
        }
    }

    #[inline(never)]
    fn error_at_origin() -> anyhow::Error {
        anyhow::anyhow!("original error marker")
    }
}
