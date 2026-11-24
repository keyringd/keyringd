use super::command_observation::CommandObservation;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolExecutionMetadata {
    pub name: String,
    pub path: Option<PathBuf>,
    pub version: Result<String, String>,
}

impl ToolExecutionMetadata {
    pub fn capture(name: &str, executable: &Path, arguments: &[&str], cwd: &Path) -> Self {
        let path = CommandObservation::resolve(executable);
        let version = match &path {
            Some(path) => CommandObservation::run(path, arguments, cwd),
            None => Err(format!("executable not found: {}", executable.display())),
        };
        Self {
            name: name.into(),
            path,
            version,
        }
    }
}
