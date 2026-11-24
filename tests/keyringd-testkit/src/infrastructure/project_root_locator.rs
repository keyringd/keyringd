use std::fs;
use std::path::{Path, PathBuf};

use crate::model::CaseResult;

pub(crate) struct ProjectRootLocator;

impl ProjectRootLocator {
    pub(crate) fn locate(results: &[CaseResult]) -> Option<PathBuf> {
        std::env::current_dir()
            .ok()
            .and_then(|directory| Self::from_directory(&directory))
            .or_else(|| {
                results.iter().find_map(|result| {
                    let source = Path::new(&result.source.file);
                    let absolute_source = if source.is_absolute() {
                        source.to_path_buf()
                    } else {
                        std::env::current_dir().ok()?.join(source)
                    };
                    Self::from_directory(&absolute_source)
                })
            })
    }
    pub(crate) fn from_directory(path: &Path) -> Option<PathBuf> {
        path.ancestors()
            .find(|directory| {
                fs::read_to_string(directory.join("Cargo.toml"))
                    .is_ok_and(|manifest| manifest.lines().any(|line| line.trim() == "[workspace]"))
            })
            .map(Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ProjectRootLocator::from_directory" {
            case "Finds the nearest workspace manifest" {
                verify "resolve the closest manifest declaring a Cargo workspace" {
                    std::thread::spawn(|| {
                        let directory = tempfile::tempdir().unwrap();
                        let source_directory = directory.path().join("crate/src");
                        fs::create_dir_all(&source_directory).unwrap();
                        fs::write(directory.path().join("Cargo.toml"), "[workspace]\n").unwrap();

                        assert_eq!(
                            ProjectRootLocator::from_directory(&source_directory),
                            Some(directory.path().to_path_buf())
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
