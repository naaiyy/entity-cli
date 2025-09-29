use serde::Serialize;
use thiserror::Error;

pub type CoreResult<T, E = CoreError> = Result<T, E>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid descriptor: {0}")]
    InvalidDescriptor(String),
    #[error("Unknown node id: {0}")]
    UnknownNode(String),
    #[error("Wrong node kind: expected {expected}, got {actual}")]
    WrongKind { expected: String, actual: String },
    #[error("Missing selections: {0:?}")]
    MissingSelections(Vec<String>),
    #[error("Invalid selection: {0}")]
    InvalidSelection(String),
    #[error("Invalid selection names: {0:?}")]
    InvalidNames(Vec<String>),
    #[error("Missing source: {0}")]
    MissingSource(String),
    #[error("Target path not found: {0}")]
    TargetNotFound(String),
    #[error("Target path not writable: {0}")]
    TargetNotWritable(String),
    #[error("Packs path not found or unreadable: {0}")]
    PacksNotFound(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Io(_) => "IO_ERROR",
            CoreError::Json(_) => "JSON_ERROR",
            CoreError::InvalidDescriptor(_) => "INVALID_DESCRIPTOR",
            CoreError::UnknownNode(_) => "UNKNOWN_NODE",
            CoreError::WrongKind { .. } => "WRONG_KIND",
            CoreError::MissingSelections(_) => "MISSING_SELECTIONS",
            CoreError::InvalidSelection(_) => "INVALID_SELECTION",
            CoreError::InvalidNames(_) => "INVALID_SELECTION",
            CoreError::MissingSource(_) => "MISSING_SOURCE",
            CoreError::TargetNotFound(_) => "TARGET_NOT_FOUND",
            CoreError::TargetNotWritable(_) => "TARGET_NOT_WRITABLE",
            CoreError::PacksNotFound(_) => "PACKS_NOT_FOUND",
        }
    }

    pub fn envelope(&self, details: Option<serde_json::Value>) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
                details,
            },
        }
    }
}
