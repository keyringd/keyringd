use std::fs;
use std::path::{Path, PathBuf};

use super::ProjectRootLocator;
use crate::model::SourceLocation;

pub(crate) struct SourceSnippetLoader;

impl SourceSnippetLoader {
    pub(crate) fn load(source: &SourceLocation, end_line: u32) -> Result<String, String> {
        let contents = fs::read_to_string(Self::source_path(source)?)
            .map_err(|error| format!("could not read {}: {error}", source.file))?;
        let lines: Vec<_> = contents.lines().collect();
        let start = line_index(source.line, "source start")?;
        let end = line_index(end_line, "source end")?;
        if start > end || end >= lines.len() {
            return Err(format!(
                "invalid source range {}..={end_line} in {}",
                source.line, source.file
            ));
        }
        Ok(lines[start..=end].join("\n"))
    }

    pub(crate) fn source_path(source: &SourceLocation) -> Result<PathBuf, String> {
        let directory = std::env::current_dir()
            .map_err(|error| format!("could not locate the test workspace: {error}"))?;
        Self::source_path_from_directory(source, &directory)
    }

    fn source_path_from_directory(
        source: &SourceLocation,
        directory: &Path,
    ) -> Result<PathBuf, String> {
        let file = Path::new(&source.file);
        if file.is_absolute() {
            return Ok(file.to_path_buf());
        }
        let workspace = ProjectRootLocator::from_directory(directory);
        workspace
            .map(|workspace| workspace.join(file))
            .ok_or_else(|| {
                format!(
                    "could not locate a Cargo workspace from {} to read {}",
                    directory.display(),
                    source.file
                )
            })
    }
}

fn line_index(line: u32, name: &str) -> Result<usize, String> {
    usize::try_from(line)
        .ok()
        .and_then(|line| line.checked_sub(1))
        .ok_or_else(|| format!("{name} line must be greater than zero"))
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "SourceSnippetLoader::load" {
            case "Loads workspace relative source when cargo runs in a crate directory" {
                verify "read a workspace-relative source path from a crate working directory" {
                    std::thread::spawn(|| {
                        let source = SourceLocation::from_manifest(env!("CARGO_MANIFEST_DIR"), file!(), 1, 1);
                        assert!(Path::new(&source.file).is_relative());
                        assert_eq!(
                            SourceSnippetLoader::load(&source, 1).unwrap(),
                            "use std::fs;"
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SourceSnippetLoader::source_path_from_directory" {
            case "Resolves sources from nested crates in a relocated workspace" {
                verify "locate the source in the relocated workspace when invoked from a nested crate" {
                    std::thread::spawn(|| {
                        let workspace = tempfile::tempdir().unwrap();
                        fs::write(
                            workspace.path().join("Cargo.toml"),
                            "[workspace]\nmembers = []\n",
                        )
                        .unwrap();
                        let directory = workspace.path().join("tests/example-crate");
                        fs::create_dir_all(directory.join("src")).unwrap();
                        fs::write(
                            directory.join("Cargo.toml"),
                            "[package]\nname = \"example-crate\"\n",
                        )
                        .unwrap();
                        fs::write(directory.join("src/case.rs"), "relocated source\n").unwrap();
                        let source = SourceLocation::new("tests/example-crate/src/case.rs", 1, 1);
                        for cwd in [workspace.path(), directory.as_path()] {
                            let path = SourceSnippetLoader::source_path_from_directory(&source, cwd).unwrap();
                            assert_eq!(fs::read_to_string(path).unwrap(), "relocated source\n");
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SourceSnippetLoader::load" {
            case "Extracts the exact source range without interpreting case syntax" {
                verify "return exactly the selected source lines without transforming their text" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let source_file = directory.path().join("case.rs");
                        fs::write(
                            &source_file,
                            "before\ncase r#\"raw title\"# {\n    verify \"works\" {}\n}\nafter\n",
                        )
                        .unwrap();
                        let source = SourceLocation::new(source_file.to_string_lossy(), 2, 1);

                        let snippet = SourceSnippetLoader::load(&source, 4).unwrap();

                        assert_eq!(
                            snippet,
                            "case r#\"raw title\"# {\n    verify \"works\" {}\n}"
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
