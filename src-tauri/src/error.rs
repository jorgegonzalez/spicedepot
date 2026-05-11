use serde::{Serialize, Serializer};

/// Single error type returned from every Tauri command.
///
/// Tauri serializes the `Err` variant of a command's `Result` by calling
/// `Serialize`. We collapse to a string so the frontend gets a clean
/// message; the full `Display` (which includes `#[from]` source chains
/// like `tonic::transport::Error`) is recorded server-side by
/// `commands::log_err`, which every command wraps its result in.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("connection not found: {0}")]
    NotFound(String),

    #[error("connection store: {0}")]
    Store(String),

    #[error("token storage (keychain): {0}")]
    Keyring(#[from] keyring::Error),

    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("transport: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("grpc {code:?}: {message}")]
    Grpc {
        code: tonic::Code,
        message: String,
    },

    #[error("tauri: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<tonic::Status> for AppError {
    fn from(s: tonic::Status) -> Self {
        AppError::Grpc {
            code: s.code(),
            message: s.message().to_string(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(format!("{e:#}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Store(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
