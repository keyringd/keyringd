use std::path::PathBuf;

/// Match Gradle's default Java selection: JAVA_HOME, then PATH.
pub(super) fn executable() -> PathBuf {
    let program = if cfg!(windows) { "java.exe" } else { "java" };
    match std::env::var_os("JAVA_HOME").filter(|home| !home.is_empty()) {
        Some(home) => PathBuf::from(home).join("bin").join(program),
        None => PathBuf::from(program),
    }
}
