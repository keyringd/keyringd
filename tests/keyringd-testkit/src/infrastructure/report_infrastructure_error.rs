use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ReportInfrastructureError {
    #[error("failed to access test result {path}: {source}")]
    ResultIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to decode test result {path}: {source}")]
    ResultDecode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to encode test result {id}: {source}")]
    ResultEncode {
        id: String,
        #[source]
        source: serde_json::Error,
    },
}
