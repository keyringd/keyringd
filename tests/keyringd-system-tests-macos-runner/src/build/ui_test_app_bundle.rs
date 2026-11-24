use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supplies the product bundle metadata that AppKit's standard About panel
/// reads. A bare Cargo executable has no application name or version metadata.
pub(super) struct UiTestAppBundle;

impl UiTestAppBundle {
    pub(super) fn prepare(workspace: &Path, executable: &Path) -> Result<PathBuf> {
        let bundle = executable
            .parent()
            .ok_or_else(|| anyhow::anyhow!("fixture has no parent directory"))?
            .join("keyringd-ui-test.app/Contents");
        let binaries = bundle.join("MacOS");
        std::fs::create_dir_all(&binaries)?;
        let metadata = std::fs::read_to_string(workspace.join("packaging/macos/Info.plist"))?
            .replace(
                "<key>CFBundleExecutable</key>\n    <string>keyringd</string>",
                "<key>CFBundleExecutable</key>\n    <string>keyringd-macos-ui-test-app</string>",
            )
            .replace("com.keyringd.Keyringd", "com.keyringd.UiTest");
        std::fs::write(bundle.join("Info.plist"), metadata)?;
        Self::prepare_icon(workspace, &bundle)?;
        let target = binaries.join(super::FIXTURE);
        std::fs::copy(executable, &target)?;
        Ok(target)
    }

    fn prepare_icon(workspace: &Path, bundle: &Path) -> Result<()> {
        let resources = bundle.join("Resources");
        std::fs::create_dir_all(&resources)?;
        let temporary = tempfile::tempdir()?;
        let iconset = temporary.path().join("keyringd.iconset");
        std::fs::create_dir(&iconset)?;
        // The same application artwork used by macOS packaging.
        let artwork = workspace.join("packaging/windows/keyringd.ico");
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
            let output = Command::new("sips")
                .args(["-s", "format", "png", "-z", &pixels, &pixels])
                .arg(&artwork)
                .arg("--out")
                .arg(iconset.join(format!("icon_{name}.png")))
                .output()
                .context("could not render the UI fixture application icon")?;
            ensure!(
                output.status.success(),
                "sips failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let output = Command::new("iconutil")
            .args(["-c", "icns"])
            .arg(&iconset)
            .arg("-o")
            .arg(resources.join("keyringd.icns"))
            .output()
            .context("could not assemble the UI fixture application icon")?;
        ensure!(
            output.status.success(),
            "iconutil failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}
