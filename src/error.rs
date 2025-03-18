use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key error: {0}")]
    KeyError(String),
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Serialization(err.to_string())
    }
}
