use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    #[doc(hidden)]
    pub fn from_manifest(manifest_directory: &str, file: &str, line: u32, column: u32) -> Self {
        let manifest_directory = std::path::Path::new(manifest_directory);
        let file = std::path::Path::new(file);
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("testkit must be inside tests in the workspace");
        Self::from_workspace(workspace, manifest_directory, file, line, column)
    }

    fn from_workspace(
        workspace: &std::path::Path,
        manifest_directory: &std::path::Path,
        file: &std::path::Path,
        line: u32,
        column: u32,
    ) -> Self {
        let path = if file.is_absolute() {
            file.strip_prefix(workspace).unwrap_or(file).to_path_buf()
        } else {
            let crate_directory = manifest_directory
                .strip_prefix(workspace)
                .expect("crate manifest must be inside the workspace");
            if file.starts_with(crate_directory) {
                file.to_path_buf()
            } else {
                crate_directory.join(file)
            }
        };
        Self::new(path.to_string_lossy().replace('\\', "/"), line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "SourceLocation::from_workspace" {
            case "Resolves workspace relative file from crate manifest" {
                verify "combine the crate manifest directory with the relative source path" {
                    std::thread::spawn(|| {
                        let manifest = std::path::Path::new("workspace").join("example-crate");
                        let file = std::path::Path::new("example-crate")
                            .join("tests")
                            .join("case.rs");

                        let location = SourceLocation::from_workspace(
                            std::path::Path::new("workspace"),
                            &manifest,
                            &file,
                            12,
                            3,
                        );

                        assert_eq!(
                            std::path::Path::new(&location.file),
                            std::path::Path::new("example-crate/tests/case.rs")
                        );
                        assert_eq!((location.line, location.column), (12, 3));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SourceLocation::from_workspace" {
            case "Removes the build machine workspace from an absolute file" {
                verify "strip the recorded build workspace prefix while preserving the file path" {
                    std::thread::spawn(|| {
                        let workspace = tempfile::tempdir().unwrap();
                        let manifest = workspace.path().join("example-crate");
                        let file = manifest.join("tests/case.rs");
                        let location = SourceLocation::from_workspace(workspace.path(), &manifest, &file, 12, 3);

                        assert_eq!(
                            std::path::Path::new(&location.file),
                            std::path::Path::new("example-crate/tests/case.rs")
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SourceLocation::from_workspace" {
            case "Preserves nested crate paths for reports" {
                verify "retain the nested package directories in the reported source path" {
                    std::thread::spawn(|| {
                        let workspace = tempfile::tempdir().unwrap();
                        let manifest = workspace.path().join("tests/example-crate");
                        for file in [
                            std::path::PathBuf::from("tests/example-crate/src/case.rs"),
                            std::path::PathBuf::from("src/case.rs"),
                            manifest.join("src/case.rs"),
                        ] {
                            let location =
                                SourceLocation::from_workspace(workspace.path(), &manifest, &file, 12, 3);
                            assert_eq!(
                                std::path::Path::new(&location.file),
                                std::path::Path::new("tests/example-crate/src/case.rs")
                            );
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
