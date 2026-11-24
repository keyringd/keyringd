use std::io;
use std::path::PathBuf;

use crate::infrastructure::ReportInfrastructureError;

#[derive(Debug, thiserror::Error)]
pub enum ReportGenerationError {
    #[error(transparent)]
    Results(#[from] ReportInfrastructureError),

    #[error(transparent)]
    Requirements(#[from] crate::requirements_report::ReportError),

    #[error("failed to create report directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to write HTML report {path}: {source}")]
    HtmlWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
