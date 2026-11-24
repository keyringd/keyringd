#[path = "../packaging/licenses/generate.rs"]
mod licenses;

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    licenses::generate()
        .expect("license generation should succeed with the dependency manifests in the workspace");
    windows_reactor_setup::as_self_contained();
    embed_windows_resources();
}

fn embed_windows_resources() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../packaging/windows/keyringd.manifest");
    let resources = manifest.with_file_name("keyringd.rc");
    let icon = resources.with_file_name("keyringd.ico");
    let output_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let output = output_dir.join("keyringd.res");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", resources.display());
    println!("cargo:rerun-if-changed={}", icon.display());

    compile_resources(&resources, &output);
    generate_thumbnail_pixels(&icon, &output_dir);
    embed_manifest(&manifest, &output_dir);
    println!("cargo:rustc-link-arg={}", output.display());
}

fn generate_thumbnail_pixels(icon: &Path, output_dir: &Path) {
    let pixels = image::open(icon)
        .expect("the Windows application icon should be readable")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    std::fs::write(output_dir.join("keyringd-256.rgba"), pixels.into_raw())
        .expect("Cargo OUT_DIR should be writable");
}

fn compile_resources(resources: &Path, output: &Path) {
    let rc = find_resource_compiler()
        .expect("rc.exe should be available because Windows builds require the Windows SDK");
    let status = Command::new(rc)
            .arg("/nologo")
            .arg(format!("/fo{}", output.display()))
            .arg(resources)
            .status()
            .expect("the resource compiler should be executable because it was located in the installed Windows SDK");
    assert!(status.success(), "Windows resource compilation failed");
}

fn embed_manifest(manifest: &Path, output_dir: &Path) {
    // mt.exe cannot reliably read manifests through the WSL UNC share. Stage the
    // unchanged product manifest alongside the native build outputs first.
    let staged_manifest = output_dir.join("keyringd.manifest");
    std::fs::copy(manifest, &staged_manifest).expect("the product manifest should be readable and Cargo OUT_DIR should be writable during the Windows build");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        staged_manifest.display()
    );
}

fn find_resource_compiler() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let program_files = std::env::var_os("ProgramFiles(x86)")?;
    let root = Path::new(&program_files).join("Windows Kits/10/bin");
    let mut versions = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    versions.sort_unstable_by(|left, right| right.file_name().cmp(&left.file_name()));
    versions
        .into_iter()
        .map(|version| version.join("x64/rc.exe"))
        .find(|candidate| candidate.is_file())
}
