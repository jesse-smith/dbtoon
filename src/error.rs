use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbtoonError {
    #[error("validation: {reason}")]
    Validation { reason: String },

    #[error("connection: {message}")]
    Connection { message: String },

    #[error("query: {message}")]
    Query { message: String },

    #[error("timeout: query timed out after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("config: {message}")]
    Config { message: String },

    #[error("auth: {message}")]
    Auth { message: String },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("format: {message}")]
    Format { message: String },
}
