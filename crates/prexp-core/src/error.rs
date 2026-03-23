use thiserror::Error;

#[derive(Debug, Error)]
pub enum FdtopError {
    #[error("process {pid} not found")]
    ProcessNotFound { pid: i32 },

    #[error("permission denied for process {pid}")]
    PermissionDenied { pid: i32 },

    #[error("backend error: {0}")]
    Backend(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
