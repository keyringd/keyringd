use anyhow::{Context, Result, bail, ensure};
use std::path::{Path, PathBuf};

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::write(&dest_path, std::fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

pub(crate) fn copy_first_rpm(rpms_dir: &Path, output_dir: &Path) -> Result<()> {
    for entry in walkdir(rpms_dir)? {
        let path = entry?;
        if path.extension().is_some_and(|ext| ext == "rpm") {
            let name = path.file_name().context("rpm has no file name")?;
            let dest = output_dir.join(name);
            std::fs::copy(&path, &dest)?;
            println!("{}", dest.display());
            return Ok(());
        }
    }
    bail!("rpmbuild did not produce an RPM")
}

fn walkdir(dir: &Path) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    let mut paths = Vec::new();
    collect_files(dir, &mut paths)?;
    Ok(paths.into_iter().map(Ok))
}

pub(crate) fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_files(&entry.path(), out)?;
        } else {
            out.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

pub(in crate::core) fn resolve_file(path: &Path, label: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)
        .with_context(|| format!("{label} not found: {}", path.display()))?;
    ensure!(path.is_file(), "not a file: {}", path.display());
    Ok(path)
}

pub(in crate::core) fn resolve_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)
        .with_context(|| format!("{label} not found: {}", path.display()))?;
    ensure!(path.is_dir(), "not a directory: {}", path.display());
    Ok(path)
}

pub(in crate::core) fn prepare_output_directory(path: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(path)?;
    Ok(std::fs::canonicalize(path)?)
}
