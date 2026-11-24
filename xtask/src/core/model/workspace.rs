use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn target(&self) -> PathBuf {
        self.root.join("target")
    }

    pub(crate) fn default_report_directory(&self) -> PathBuf {
        self.target().join("test-reports")
    }
}
