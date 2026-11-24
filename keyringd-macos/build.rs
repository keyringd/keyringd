#[path = "../packaging/licenses/generate.rs"]
mod licenses;
fn main() -> anyhow::Result<()> {
    println!("cargo:rustc-link-arg=-mmacosx-version-min=13.0");
    licenses::generate()
}
