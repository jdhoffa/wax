use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::cli::Cli;
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub cache_dir: Option<PathBuf>,
    pub request_delay_ms: Option<u64>,
    pub concurrency: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub user_agent: Option<String>,
    pub max_collectors: Option<usize>,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub cache_dir: PathBuf,
    pub request_delay_ms: u64,
    pub concurrency: usize,
    pub timeout_ms: u64,
    pub user_agent: String,
}

impl Settings {
    pub fn load(cli: &Cli) -> Result<Self> {
        let file_cfg = load_file_config(cli.config.as_deref())?;
        let project_dirs = ProjectDirs::from("dev", "jdhoffa", "wax").ok_or_else(|| {
            AppError::InvalidInput("unable to determine cache directory".to_string())
        })?;

        let cache_dir = cli
            .cache_dir
            .clone()
            .or(file_cfg.cache_dir)
            .unwrap_or_else(|| project_dirs.cache_dir().to_path_buf());

        Ok(Self {
            cache_dir,
            request_delay_ms: cli
                .rate_limit_ms
                .or(file_cfg.request_delay_ms)
                .unwrap_or(750),
            concurrency: cli.concurrency.or(file_cfg.concurrency).unwrap_or(4),
            timeout_ms: cli.timeout_ms.or(file_cfg.timeout_ms).unwrap_or(10_000),
            user_agent: cli
                .user_agent
                .clone()
                .or(file_cfg.user_agent)
                .unwrap_or_else(|| "wax/0.1".to_string()),
        })
    }
}

fn load_file_config(path: Option<&Path>) -> Result<FileConfig> {
    let Some(path) = path else {
        return Ok(FileConfig {
            cache_dir: None,
            request_delay_ms: None,
            concurrency: None,
            timeout_ms: None,
            user_agent: None,
            max_collectors: None,
            max_depth: None,
        });
    };

    let text = std::fs::read_to_string(path)?;
    let cfg = toml::from_str(&text)?;
    Ok(cfg)
}
