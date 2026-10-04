use thiserror::Error;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No saved session — run `matrix-bridge setup` first")]
    NoSession,

    #[error("Matrix error: {0}")]
    Matrix(String),

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Sync failed: {0}")]
    SyncFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Key import failed: {0}")]
    ImportFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BridgeError>;
