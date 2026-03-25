use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    fetched_at: u64,
    body: String,
}

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub async fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn get(&self, url: &str, max_age_ms: u64) -> Result<Option<String>> {
        let path = self.entry_path(url);
        let Ok(raw) = fs::read_to_string(path).await else {
            return Ok(None);
        };

        let entry: CacheEntry = serde_json::from_str(&raw)?;
        let now = now_ms();
        if now.saturating_sub(entry.fetched_at) > max_age_ms {
            return Ok(None);
        }

        Ok(Some(entry.body))
    }

    pub async fn put(&self, url: &str, body: &str) -> Result<()> {
        let entry = CacheEntry {
            url: url.to_string(),
            fetched_at: now_ms(),
            body: body.to_string(),
        };

        let raw = serde_json::to_string(&entry)?;
        fs::write(self.entry_path(url), raw).await?;
        Ok(())
    }

    pub async fn stats(&self) -> Result<(usize, u64)> {
        let mut entries = 0usize;
        let mut bytes = 0u64;
        let mut read_dir = fs::read_dir(&self.root).await?;

        while let Some(item) = read_dir.next_entry().await? {
            let meta = item.metadata().await?;
            if meta.is_file() {
                entries += 1;
                bytes += meta.len();
            }
        }

        Ok((entries, bytes))
    }

    pub async fn clear(&self) -> Result<()> {
        let mut read_dir = fs::read_dir(&self.root).await?;
        while let Some(item) = read_dir.next_entry().await? {
            if item.metadata().await?.is_file() {
                fs::remove_file(item.path()).await?;
            }
        }
        Ok(())
    }

    fn entry_path(&self, url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        self.root.join(format!("{digest}.json"))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
