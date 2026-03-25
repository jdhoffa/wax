use std::path::PathBuf;
use std::time::Duration;

use reqwest::Client;
use tokio::time::sleep;

use crate::cache::{Cache, CacheStats};
use crate::config::Settings;
use crate::error::{AppError, Result};

#[derive(Debug)]
pub struct Fetcher {
    client: Client,
    cache: Cache,
    request_delay_ms: u64,
    timeout_ms: u64,
    pub stats: CacheStats,
}

impl Fetcher {
    pub async fn new(settings: &Settings) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(settings.timeout_ms))
            .user_agent(settings.user_agent.clone())
            .build()?;

        Ok(Self {
            client,
            cache: Cache::new(settings.cache_dir.clone()).await?,
            request_delay_ms: settings.request_delay_ms,
            timeout_ms: settings.timeout_ms,
            stats: CacheStats::default(),
        })
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    pub async fn fetch_text(&mut self, url: &str) -> Result<String> {
        if let Some(path) = file_path_from_url(url) {
            return Ok(tokio::fs::read_to_string(path).await?);
        }

        if let Some(body) = self.cache.get(url, self.timeout_ms * 60).await? {
            self.stats.hits += 1;
            return Ok(body);
        }

        self.stats.misses += 1;
        sleep(Duration::from_millis(self.request_delay_ms)).await;
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Network(format!(
                "request failed for {url}: {status}"
            )));
        }

        let body = response.text().await?;
        self.cache.put(url, &body).await?;
        Ok(body)
    }
}

fn file_path_from_url(url: &str) -> Option<PathBuf> {
    url.strip_prefix("file://").map(PathBuf::from)
}
