//! Direct HTTP download fallback method
//!
//! This module provides a fallback to direct HTTP streaming for:
//! - Unsupported sources
//! - Direct URL downloads
//! - Progressive download with resume capability

use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;

pub struct HttpDownloader {
    client: Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn get_stream_url(&self, url: &str) -> Result<String> {
        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("Invalid URL format"));
        }

        // Check if URL is accessible
        let response = self.client.head(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("URL not accessible: {}", response.status()));
        }

        // Return the URL as-is for direct streaming
        Ok(url.to_string())
    }

    pub async fn get_stream_info(&self, url: &str) -> Result<StreamInfo> {
        let response = self.client.head(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get stream info: {}", response.status()));
        }

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();

        Ok(StreamInfo {
            url: url.to_string(),
            content_length,
            content_type,
        })
    }

    pub async fn download_range(&self, url: &str, start: u64, end: u64) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .header("Range", format!("bytes={}-{}", start, end))
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(anyhow!("Range request failed: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct StreamInfo {
    pub url: String,
    pub content_length: u64,
    pub content_type: String,
}

/// Parse quality from URL for direct HTTP downloads
pub fn parse_quality_from_url(url: &str) -> Option<String> {
    // Common quality patterns in URLs
    if url.contains("_320.") {
        Some("320kbps".to_string())
    } else if url.contains("_160.") {
        Some("160kbps".to_string())
    } else if url.contains("_128.") {
        Some("128kbps".to_string())
    } else if url.contains("_96.") {
        Some("96kbps".to_string())
    } else {
        None
    }
}

/// Extract file extension from URL
pub fn extract_extension(url: &str) -> Option<String> {
    url.split('.').last().map(|s| s.split('?').next().unwrap_or(s).to_string())
}
