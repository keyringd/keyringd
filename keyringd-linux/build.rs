#[path = "../packaging/licenses/generate.rs"]
mod licenses;

fn main() -> anyhow::Result<()> {
    licenses::generate()
}
