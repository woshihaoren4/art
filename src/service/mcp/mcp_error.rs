use thiserror::Error;
#[derive(Error, Debug)]
pub enum McpError {
    #[error("[McpError] serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    #[error("[McpError] c timeout.")]
    Timeout,
    
    #[error("[McpError] io error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("[McpError] unknown error: {0}")]
    Unknown(anyhow::Error)
}