use std::process::ExitCode;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("{platform} does not support `{feature}` yet")]
    UnsupportedPlatformFeature { platform: String, feature: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("no usable public data found")]
    NoPublicData,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NoPublicData => 2,
            _ => 1,
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::Parse(value.to_string())
    }
}

impl From<ExitCode> for AppError {
    fn from(_: ExitCode) -> Self {
        Self::Parse("process exited unexpectedly".to_string())
    }
}
