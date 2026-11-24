use crate::core::driver::{
    linux_package, macos_package, package_files, windows_package, wsl_package,
};
use crate::core::model::{DistFormat, DistTarget, DistributionRequest, Workspace};
use anyhow::{Context, Result, bail, ensure};
use std::path::PathBuf;

pub(crate) fn run(workspace: &Workspace, command: &DistributionRequest) -> Result<()> {
    command.validate()?;
    let binary = package_files::resolve_file(&command.binary, "binary")?;
    let output_dir = package_files::prepare_output_directory(&command.output_dir)?;

    match (&command.target, &command.format) {
        (DistTarget::Macos, DistFormat::App) => {
            macos_package::build(workspace, &binary, &output_dir, command.ad_hoc)
        }
        (DistTarget::Windows, DistFormat::Runtime) => {
            ensure!(cfg!(windows), "Windows runtime packaging requires Windows");
            windows_package::stage_runtime(
                binary.parent().context("binary has no parent")?,
                &output_dir.join("windows-runtime"),
            )
        }
        (DistTarget::Windows, DistFormat::Msi) => {
            windows_package::build_msi(workspace, &binary, &output_dir)
        }
        (DistTarget::Linux, DistFormat::Deb) => {
            linux_package::build_deb(workspace, &binary, &output_dir)
        }
        (DistTarget::Linux, DistFormat::Rpm) => {
            linux_package::build_rpm(workspace, &binary, &output_dir)
        }
        (DistTarget::Wsl, DistFormat::Deb) => {
            let (windows_exe, windows_runtime_dir) = wsl_inputs(command)?;
            wsl_package::build_deb(
                workspace,
                &binary,
                &windows_exe,
                &windows_runtime_dir,
                &output_dir,
            )
        }
        (DistTarget::Wsl, DistFormat::Rpm) => {
            let (windows_exe, windows_runtime_dir) = wsl_inputs(command)?;
            wsl_package::build_rpm(
                workspace,
                &binary,
                &windows_exe,
                &windows_runtime_dir,
                &output_dir,
            )
        }
        _ => bail!("unsupported package target and format"),
    }
}

fn wsl_inputs(command: &DistributionRequest) -> Result<(PathBuf, PathBuf)> {
    let (windows_exe, windows_runtime_dir) = command.windows_inputs()?;
    Ok((
        package_files::resolve_file(windows_exe, "windows exe")?,
        package_files::resolve_directory(windows_runtime_dir, "windows runtime dir")?,
    ))
}
