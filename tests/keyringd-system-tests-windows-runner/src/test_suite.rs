use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum TestSuite {
    Windows,
    Wsl {
        distribution: String,
        package_directory: PathBuf,
    },
}
