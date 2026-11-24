use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::OnceLock;

const SIGNALS: [(libc::c_int, &str); 6] = [
    (libc::SIGABRT, "SIGABRT"),
    (libc::SIGBUS, "SIGBUS"),
    (libc::SIGFPE, "SIGFPE"),
    (libc::SIGILL, "SIGILL"),
    (libc::SIGSEGV, "SIGSEGV"),
    (libc::SIGTRAP, "SIGTRAP"),
];

static LOG: OnceLock<FatalSignalLog> = OnceLock::new();

/// Keeps a file and preformatted messages ready for a fatal signal. The handler
/// must not allocate, lock, format messages, or call the application logger.
pub(crate) struct FatalSignalLog {
    file: File,
    messages: [Vec<u8>; SIGNALS.len()],
    previous: [libc::sigaction; SIGNALS.len()],
}

impl FatalSignalLog {
    pub(crate) fn install(directory: &Path) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(directory.join("fatal-signals.log"))?;
        let messages = SIGNALS.map(|(signal, name)| {
            format!(
                "Fatal signal received: {name} ({signal}) pid={} version={}; see macOS DiagnosticReports for the cause and stack.\n",
                std::process::id(),
                env!("CARGO_PKG_VERSION"),
            )
            .into_bytes()
        });
        let mut previous = unsafe { std::mem::zeroed::<[libc::sigaction; SIGNALS.len()]>() };
        for (index, (signal, _)) in SIGNALS.iter().enumerate() {
            if unsafe { libc::sigaction(*signal, std::ptr::null(), &mut previous[index]) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        LOG.set(Self {
            file,
            messages,
            previous,
        })
        .map_err(|_| io::Error::other("fatal signal logging already installed"))?;
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = record_signal as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        unsafe { libc::sigemptyset(&mut action.sa_mask) };
        for (index, (signal, _)) in SIGNALS.iter().enumerate() {
            if unsafe { libc::sigaction(*signal, &action, std::ptr::null_mut()) } != 0 {
                let error = io::Error::last_os_error();
                // Do not leave a partially installed set of handlers on failure.
                for (previous_index, (installed, _)) in SIGNALS[..index].iter().enumerate() {
                    unsafe {
                        libc::sigaction(*installed, &previous[previous_index], std::ptr::null_mut())
                    };
                }
                return Err(error);
            }
        }
        Ok(())
    }
}

unsafe extern "C" fn record_signal(
    signal: libc::c_int,
    _info: *mut libc::siginfo_t,
    _context: *mut libc::c_void,
) {
    if let Some(log) = LOG.get()
        && let Some(index) = SIGNALS.iter().position(|(s, _)| *s == signal)
    {
        let message = &log.messages[index];
        unsafe {
            // One unbuffered, best-effort write to a descriptor opened at startup.
            libc::write(log.file.as_raw_fd(), message.as_ptr().cast(), message.len());
            // Forward to the original disposition, including Rust's existing
            // stack-overflow handlers, rather than swallowing the fatal signal.
            libc::sigaction(signal, &log.previous[index], std::ptr::null_mut());
            libc::raise(signal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    keyringd_testkit::scenario! {
        group "FatalSignalLog::install" {
            case "Records signal and preserves termination" {
                verify "record signal diagnostics and preserve the prior termination behavior" {
                    const CHILD: &str = "KEYRINGD_FATAL_SIGNAL_TEST";
                    if let Ok(mode) = std::env::var(CHILD) {
                        let directory = std::env::var_os("KEYRINGD_FATAL_SIGNAL_DIRECTORY").unwrap();
                        let signal = if mode == "abort" {
                            libc::SIGABRT
                        } else {
                            libc::SIGTRAP
                        };
                        if mode == "previous" {
                            unsafe { libc::signal(signal, previous_handler as *const () as libc::sighandler_t) };
                        }
                        FatalSignalLog::install(Path::new(&directory)).unwrap();
                        if mode == "worker" {
                            std::thread::spawn(move || unsafe { libc::raise(signal) })
                                .join()
                                .unwrap();
                        } else {
                            unsafe { libc::raise(signal) };
                        }
                        panic!("fatal signal unexpectedly returned");
                    }

                    let directory = tempfile::tempdir().unwrap();
                    for mode in ["trap", "worker", "abort", "previous"] {
                        let output = std::process::Command::new(std::env::current_exe().unwrap())
                            .current_dir(directory.path())
                            .args([
                                "--exact",
                                "system::fatal_signal_log::tests::records_signal_and_preserves_termination",
                                "--nocapture",
                            ])
                            .env(CHILD, mode)
                            .env("KEYRINGD_FATAL_SIGNAL_DIRECTORY", directory.path())
                            .env("RUST_LOG", "off")
                            .spawn()
                            .unwrap()
                            .wait_with_output()
                            .unwrap();
                        if mode == "previous" {
                            assert_eq!(output.status.code(), Some(77));
                        } else {
                            assert_eq!(
                                output.status.signal(),
                                Some(if mode == "abort" {
                                    libc::SIGABRT
                                } else {
                                    libc::SIGTRAP
                                })
                            );
                        }
                        let log = std::fs::read_to_string(directory.path().join("fatal-signals.log")).unwrap();
                        let signal_name = if mode == "abort" {
                            "SIGABRT"
                        } else {
                            "SIGTRAP"
                        };
                        assert!(log.contains(signal_name), "{log}");
                        assert!(log.contains("pid="), "{log}");
                        assert!(log.contains("DiagnosticReports"), "{log}");
                    }
                    let log = std::fs::read_to_string(directory.path().join("fatal-signals.log")).unwrap();
                    assert_eq!(log.lines().count(), 4, "{log}");
                }
            }
        }
    }

    unsafe extern "C" fn previous_handler(_: libc::c_int) {
        unsafe { libc::_exit(77) };
    }
}
