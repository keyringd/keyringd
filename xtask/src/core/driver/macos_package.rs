use crate::core::driver::macos_bundle::set_bundle_version;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};
use plist::Value;

use super::workspace_manifest::read_version;
use crate::core::driver::MacosTools;
use crate::core::model::Workspace;

pub(crate) fn build(
    workspace: &Workspace,
    binary: &Path,
    output: &Path,
    ad_hoc: bool,
) -> Result<()> {
    ensure!(cfg!(target_os = "macos"), "macOS packaging requires macOS");
    let mut tools = MacosTools::new()?;
    let version = read_version(workspace)?;
    // Stage on the destination filesystem; do not replace an existing bundle until validated.
    let staging = tempfile::tempdir_in(output)?;
    let app = staging.path().join("keyringd.app");
    assemble(workspace, binary, &app, &version, &tools)?;
    if !ad_hoc {
        return super::macos_release::create_notarized_archive(&app, &version, output, &mut tools);
    }
    tools.checked(
        Command::new("codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .arg(&app),
    )?;
    tools.checked(
        Command::new("codesign")
            .args(["--verify", "--strict"])
            .arg(&app),
    )?;
    let name = "keyringd.app";
    let destination = output.join(name);
    if destination.exists() {
        std::fs::remove_dir_all(&destination)?;
    }
    std::fs::rename(staging.path().join(name), &destination)?;
    println!("{}", destination.display());
    Ok(())
}

fn assemble(
    workspace: &Workspace,
    binary: &Path,
    app: &Path,
    version: &semver::Version,
    tools: &MacosTools,
) -> Result<()> {
    let packaging = workspace.root().join("packaging/macos");
    let contents = app.join("Contents");
    let resources = contents.join("Resources");
    std::fs::create_dir_all(contents.join("MacOS"))?;
    std::fs::create_dir_all(&resources)?;
    let mut info = Value::from_file(packaging.join("Info.plist"))?;
    set_bundle_version(&mut info, version)?;
    info.to_file_xml(contents.join("Info.plist"))?;
    let executable = contents.join("MacOS/keyringd");
    std::fs::copy(binary, &executable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }

    // The shared ICO is also used by the UI-test bundle; sips supports it on macOS.
    let icon_source = workspace.root().join("packaging/windows/keyringd.ico");
    let iconset = app
        .parent()
        .context("bundle has no parent")?
        .join("keyringd.iconset");
    std::fs::create_dir(&iconset)?;
    for (pixels, name) in [
        (16, "16x16"),
        (32, "16x16@2x"),
        (32, "32x32"),
        (64, "32x32@2x"),
        (128, "128x128"),
        (256, "128x128@2x"),
        (256, "256x256"),
        (512, "256x256@2x"),
        (512, "512x512"),
        (1024, "512x512@2x"),
    ] {
        let pixels = pixels.to_string();
        tools.checked(
            Command::new("sips")
                .args(["-s", "format", "png", "-z", &pixels, &pixels])
                .arg(&icon_source)
                .arg("--out")
                .arg(iconset.join(format!("icon_{name}.png"))),
        )?;
    }
    let icon = resources.join("keyringd.icns");
    tools.checked(
        Command::new("iconutil")
            .args(["-c", "icns"])
            .arg(&iconset)
            .arg("-o")
            .arg(&icon),
    )?;
    ensure!(std::fs::metadata(icon)?.len() > 0, "empty bundle icon");
    tools.checked(
        Command::new("plutil")
            .arg("-lint")
            .arg(contents.join("Info.plist")),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for directory in [
            app.to_path_buf(),
            contents.clone(),
            contents.join("MacOS"),
            resources.clone(),
        ] {
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755))?;
        }
        for file in [contents.join("Info.plist"), resources.join("keyringd.icns")] {
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644))?;
        }
    }
    Ok(())
}
