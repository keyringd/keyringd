use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};
use semver::Version;
use sha2::{Digest, Sha256};

use super::{
    package_files::copy_dir_recursive, process::run_command, workspace_manifest::read_version,
};
use crate::core::model::Workspace;

pub(crate) fn build_msi(workspace: &Workspace, binary: &Path, output: &Path) -> Result<()> {
    ensure!(cfg!(windows), "MSI packaging requires Windows");
    let runtime = output.join("windows-runtime");
    stage_runtime(binary.parent().context("binary has no parent")?, &runtime)?;
    let version = read_version(workspace)?;
    let intermediate = workspace.target().join("package/windows-msi");
    std::fs::create_dir_all(&intermediate)?;
    let files = intermediate.join("GeneratedFiles.wxi");
    std::fs::write(&files, runtime_components(&runtime)?)?;

    run_command(
        Command::new("dotnet")
            .args(["tool", "restore"])
            .current_dir(workspace.root().join("packaging/windows")),
    )?;
    for extension in ["WixToolset.UI.wixext/7.0.0", "WixToolset.Util.wixext/7.0.0"] {
        wix(workspace, &["extension", "add", extension])?;
    }
    compile_msi(
        workspace,
        &runtime,
        &version,
        &files,
        &output.join(format!("keyringd-{version}-x86_64.msi")),
    )?;
    Ok(())
}

pub(crate) fn stage_runtime(source: &Path, destination: &Path) -> Result<()> {
    ensure!(
        source.join("keyringd.exe").is_file(),
        "keyringd.exe is missing from {}",
        source.display()
    );
    ensure!(
        source != destination && !source.starts_with(destination),
        "runtime staging would overwrite the input"
    );
    if destination.exists() {
        std::fs::remove_dir_all(destination)?;
    }
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let path = entry.path();
        // The SDK payload and WinUI/Windows resource loaders disagree on casing.
        // Windows ignores this difference; WSL's Linux filesystem does not.
        // EnsurePlatformResourceModuleHandle needs the exact 19H1 name (otherwise
        // E_UNEXPECTED), LoadResourceSatelliteDlls needs the common DLL name
        // (otherwise ERROR_FILE_NOT_FOUND), and MUI lookup needs en-US (otherwise
        // ERROR_MUI_FILE_NOT_LOADED). These are exercised by the WSL profile E2E.
        // https://github.com/microsoft/microsoft-ui-xaml/blob/main/dxaml/xcp/dxaml/dllsrv/winrt/dllentry.cpp
        // https://github.com/microsoft/microsoft-ui-xaml/blob/main/dxaml/xcp/host/win/desktop/BrowserHost.cpp
        let to = match name.as_str() {
            "microsoft.ui.xaml.resources.19h1.dll" => {
                destination.join("Microsoft.UI.Xaml.Resources.19H1.dll")
            }
            "microsoft.ui.xaml.resources.common.dll" => {
                destination.join("Microsoft.UI.Xaml.resources.common.dll")
            }
            "en-us" => destination.join("en-US"),
            _ => destination.join(entry.file_name()),
        };
        if entry.file_type()?.is_dir() {
            if matches!(name.as_str(), "en-us" | "microsoft.ui.xaml") {
                copy_dir_recursive(&path, &to)?;
            }
        } else if name == "keyringd.exe"
            || name == "keyringd.pdb"
            || path.extension().is_some_and(|extension| {
                matches!(
                    extension.to_string_lossy().to_ascii_lowercase().as_str(),
                    "dll" | "pri" | "xaml"
                )
            })
        {
            std::fs::copy(path, to)?;
        }
    }
    Ok(())
}

fn wix(workspace: &Workspace, arguments: &[&str]) -> Result<()> {
    run_command(
        Command::new("dotnet")
            .args(["tool", "run", "wix", "--", "-acceptEula", "wix7"])
            .args(arguments)
            .current_dir(workspace.root().join("packaging/windows")),
    )
}

fn compile_msi(
    workspace: &Workspace,
    runtime: &Path,
    version: &Version,
    files: &Path,
    output: &Path,
) -> Result<()> {
    ensure!(
        version.major <= 255 && version.minor <= 255 && version.patch <= 65535,
        "version is outside Windows Installer limits: {version}"
    );
    let msi_version = format!("{}.{}.{}", version.major, version.minor, version.patch);
    let product_code = stable_guid(&Sha256::digest(format!(
        "keyringd:windows:x64:{msi_version}"
    )));
    run_command(
        Command::new("dotnet")
            .args([
                "tool",
                "run",
                "wix",
                "--",
                "build",
                "-acceptEula",
                "wix7",
                "-arch",
                "x64",
                "-ext",
                "WixToolset.UI.wixext",
                "-ext",
                "WixToolset.Util.wixext",
            ])
            .arg("-d")
            .arg(format!("RuntimeDirectory={}", windows_path(runtime)))
            .arg("-d")
            .arg(format!("MsiVersion={msi_version}"))
            .arg("-d")
            .arg(format!("MsiProductCode={product_code}"))
            .arg("-d")
            .arg(format!(
                "IconPath={}",
                windows_path(&workspace.root().join("packaging/windows/keyringd.ico"))
            ))
            .arg("-d")
            .arg(format!("GeneratedFiles={}", windows_path(files)))
            .arg("-intermediateFolder")
            .arg(windows_path(
                &workspace.target().join("package/windows-msi"),
            ))
            .arg("-o")
            // WiX creates a System.Uri for its output. Rust canonicalization
            // adds the verbatim prefix, which .NET cannot parse as a file URI.
            .arg(windows_path(output))
            .arg(windows_path(
                &workspace.root().join("packaging/windows/msi/Package.wxs"),
            ))
            .current_dir(workspace.root().join("packaging/windows")),
    )?;
    let output = windows_path(output);
    wix(
        workspace,
        &[
            "msi", "validate", "-sice", "ICE64", "-sice", "ICE91", &output,
        ],
    )?;
    println!("{output}");
    Ok(())
}

fn runtime_components(runtime: &Path) -> Result<String> {
    let mut paths = Vec::new();
    super::package_files::collect_files(runtime, &mut paths)?;
    paths.sort();
    let mut directories = BTreeSet::new();
    let mut xml = String::from("<Include xmlns=\"http://wixtoolset.org/schemas/v4/wxs\">\n");
    for file in paths {
        let relative = file.strip_prefix(runtime)?;
        if relative == Path::new("keyringd.exe") {
            continue;
        }
        let hash = Sha256::digest(relative.to_string_lossy().replace('\\', "/").to_lowercase());
        let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
        let guid = stable_guid(&hash);
        let directory = relative
            .parent()
            .filter(|path| !path.as_os_str().is_empty());
        let subdirectory = directory
            .map(|path| {
                directories.insert(path.to_path_buf());
                format!(" Subdirectory=\"{}\"", xml_escape(&windows_path(path)))
            })
            .unwrap_or_default();
        writeln!(
            xml,
            "  <Component Id=\"RuntimeFile_{}\" Directory=\"INSTALLFOLDER\"{subdirectory} Guid=\"{guid}\">",
            &hex[..24]
        )?;
        writeln!(
            xml,
            "    <File Source=\"{}\" Name=\"{}\" />",
            xml_escape(&windows_path(&file)),
            xml_escape(
                &file
                    .file_name()
                    .context("file has no name")?
                    .to_string_lossy()
            )
        )?;
        writeln!(
            xml,
            "    <RegistryValue Root=\"HKCU\" Key=\"Software\\keyringd\\Installer\\Files\" Name=\"{hex}\" Type=\"integer\" Value=\"1\" KeyPath=\"yes\" />\n  </Component>"
        )?;
    }
    for directory in directories {
        let hash = Sha256::digest(format!(
            "directory:{}",
            directory
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase()
        ));
        let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
        let guid = stable_guid(&hash);
        let id = format!("RuntimeDirectory_{}", &hex[..24]);
        writeln!(
            xml,
            "  <Component Id=\"{id}\" Directory=\"INSTALLFOLDER\" Subdirectory=\"{}\" Guid=\"{guid}\">",
            xml_escape(&windows_path(&directory))
        )?;
        writeln!(
            xml,
            "    <RemoveFolder Id=\"Remove_{id}\" On=\"uninstall\" />"
        )?;
        writeln!(
            xml,
            "    <RegistryValue Root=\"HKCU\" Key=\"Software\\keyringd\\Installer\\Directories\" Name=\"{hex}\" Type=\"integer\" Value=\"1\" KeyPath=\"yes\" />\n  </Component>"
        )?;
    }
    xml.push_str("</Include>\n");
    Ok(xml)
}

fn stable_guid(hash: &[u8]) -> String {
    let mut bytes: [u8; 16] = hash[..16].try_into().expect("SHA256 has 32 bytes");
    bytes[7] = (bytes[7] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

fn windows_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "stage_runtime" {
            case "Staged runtime contains required resources and excludes build outputs" {
                verify "stage the runtime resources without copying compiler and test artifacts" {
                    let root = tempfile::tempdir().unwrap();
                    let source = root.path().join("build");
                    for directory in ["en-us", "Microsoft.UI.Xaml", "ja-jp", "deps"] {
                        std::fs::create_dir_all(source.join(directory)).unwrap();
                        std::fs::write(source.join(directory).join("resource.pri"), b"resource").unwrap();
                    }
                    for file in [
                        "keyringd.exe",
                        "keyringd.pdb",
                        "runtime.dll",
                        "Microsoft.ui.xaml.resources.19h1.dll",
                        "Microsoft.ui.xaml.resources.common.dll",
                        "resources.pri",
                        "view.xaml",
                        "other.exe",
                        "debug.pdb",
                    ] {
                        std::fs::write(source.join(file), b"runtime").unwrap();
                    }
                    let staged = root.path().join("stage");
                    stage_runtime(&source, &staged).unwrap();
                    for name in [
                        "keyringd.exe",
                        "keyringd.pdb",
                        "runtime.dll",
                        "Microsoft.UI.Xaml.Resources.19H1.dll",
                        "Microsoft.UI.Xaml.resources.common.dll",
                        "resources.pri",
                        "view.xaml",
                        "en-US/resource.pri",
                        "Microsoft.UI.Xaml/resource.pri",
                    ] {
                        assert!(staged.join(name).is_file(), "missing {name}");
                    }
                    for name in ["debug.pdb", "other.exe", "deps", "ja-jp"] {
                        assert!(!staged.join(name).exists(), "unexpected {name}");
                    }
                    // Inspect the stored name rather than case-insensitive Windows file lookup.
                    let names = std::fs::read_dir(&staged)
                        .unwrap()
                        .map(|entry| entry.unwrap().file_name())
                        .collect::<Vec<_>>();
                    assert!(names.contains(&"Microsoft.UI.Xaml.Resources.19H1.dll".into()));
                    assert!(!names.contains(&"Microsoft.ui.xaml.resources.19h1.dll".into()));
                    assert!(names.contains(&"Microsoft.UI.Xaml.resources.common.dll".into()));
                    assert!(!names.contains(&"Microsoft.ui.xaml.resources.common.dll".into()));
                    assert!(names.contains(&"en-US".into()));
                    assert!(!names.contains(&"en-us".into()));
                    let xml = runtime_components(&staged).unwrap();
                    assert!(!xml.contains("keyringd.exe"));
                    assert!(xml.contains("RemoveFolder"));
                    assert_eq!(xml, runtime_components(&staged).unwrap());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "stage_runtime" {
            case "Missing executable does not remove existing staging" {
                verify "retain the existing runtime directory when the requested executable is missing" {
                    let root = tempfile::tempdir().unwrap();
                    let stage = root.path().join("stage");
                    std::fs::create_dir(&stage).unwrap();
                    std::fs::write(stage.join("keep"), b"keep").unwrap();
                    assert!(stage_runtime(root.path(), &stage).is_err());
                    assert!(stage.join("keep").exists());
                }
            }
        }
    }
}
