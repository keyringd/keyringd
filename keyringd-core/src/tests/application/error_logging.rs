use crate::application::ApplicationError;

keyringd_testkit::scenario! {
    group "ApplicationError logging integration" {
        case "Writes the failure severity category and diagnostic cause to the log" {
            given "a file logger accepting info events and a security key timeout with a diagnostic cause" {
                let file = tempfile::NamedTempFile::new().unwrap();
                let subscriber = tracing_subscriber::fmt()
                    .without_time()
                    .with_ansi(false)
                    .with_target(false)
                    .with_max_level(tracing::Level::INFO)
                    .with_writer(std::sync::Mutex::new(file.reopen().unwrap()))
                    .finish();
                let error = ApplicationError::PasskeySecurityKeyTimedOut("deadline reached".into());
            }
            when "the application reports the failure" {
                tracing::subscriber::with_default(subscriber, || {
                    error.log_failure("authentication deadline expired");
                });
                let output = std::fs::read_to_string(file.path()).unwrap();
            }
            then "the log contains the info event with the operation error category and cause" {
                assert!(output.lines().any(|line| {
                    line.contains("INFO")
                        && line.contains("authentication deadline expired")
                        && line.contains("PasskeySecurityKeyTimedOut")
                        && line.contains("deadline reached")
                }), "{output}");
            }
        }
    }
}
