use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Bounded, read-only command observation for report metadata. A missing tool
/// or failed query is retained as a diagnostic rather than blocking tests.
pub(crate) struct CommandObservation;

impl CommandObservation {
    pub(crate) fn run(program: &Path, arguments: &[&str], cwd: &Path) -> Result<String, String> {
        let output = tempfile::tempfile().map_err(|error| error.to_string())?;
        let mut child = Command::new(program)
            .args(arguments)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|error| error.to_string())?)
            .stderr(output.try_clone().map_err(|error| error.to_string())?)
            .spawn()
            .map_err(|error| error.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                outcome => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(match outcome {
                        Err(error) => error.to_string(),
                        _ => "metadata query timed out".into(),
                    });
                }
            }
        };
        use std::io::{Read, Seek};
        let mut output = output;
        output.rewind().map_err(|error| error.to_string())?;
        let mut bytes = Vec::new();
        output
            .take(64 * 1024)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        let text = String::from_utf8_lossy(&bytes).trim().to_owned();
        if status.success() {
            Ok(text)
        } else {
            Err(format!("{status}: {text}"))
        }
    }

    pub(crate) fn resolve(program: &Path) -> Option<PathBuf> {
        if program.is_absolute() || program.components().count() > 1 {
            return program.canonicalize().ok();
        }
        let paths = std::env::var_os("PATH")?;
        let extensions = if cfg!(windows) {
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".EXE;.COM;.BAT;.CMD".into())
                .split(';')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for directory in std::env::split_paths(&paths) {
            let candidate = directory.join(program);
            if candidate.is_file() {
                return candidate.canonicalize().ok();
            }
            for extension in &extensions {
                let candidate = directory.join(format!("{}{}", program.display(), extension));
                if candidate.is_file() {
                    return candidate.canonicalize().ok();
                }
            }
        }
        None
    }
}
