use std::path::{Component, Path, PathBuf, Prefix};

/// cmd.exe and Gradle's batch wrapper require ordinary Win32 paths.
/// Verbatim paths disable slash normalization used by gradlew.bat's JAVA_HOME.
pub(super) fn batch_path(path: &Path) -> PathBuf {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_owned();
    };
    let mut result = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => PathBuf::from(r"\\").join(server).join(share),
        _ => return path.to_owned(),
    };
    for component in components {
        if component != Component::RootDir {
            result.push(component.as_os_str());
        }
    }
    result
}
