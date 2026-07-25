//! Direct HTTP download fallback method
//!
//! This module provides a fallback to direct HTTP streaming for:
//! - Unsupported sources
//! - Direct URL downloads
//! - Progressive download with resume capability

use anyhow::{anyhow, Result};
use bex_core::resolver::component::content_resolver::utils::{
    http_request, HttpMethod, RequestOptions,
};

pub struct HttpDownloader;

impl HttpDownloader {
    pub fn new() -> Self {
        Self
    }

    pub fn get_stream_url(&self, url: &str) -> Result<String> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("Invalid URL format"));
        }

        let options = RequestOptions {
            method: HttpMethod::Head,
            headers: Some(vec![(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            )]),
            body: None,
            timeout_seconds: Some(10),
        };

        let response = http_request(url, &options).map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if response.status < 200 || response.status >= 400 {
            return Err(anyhow!("URL not accessible: {}", response.status));
        }

        Ok(url.to_string())
    }

    #[allow(dead_code)]
    pub fn get_stream_info(&self, url: &str) -> Result<StreamInfo> {
        let options = RequestOptions {
            method: HttpMethod::Head,
            headers: Some(vec![(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            )]),
            body: None,
            timeout_seconds: Some(10),
        };

        let response = http_request(url, &options).map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if response.status < 200 || response.status >= 400 {
            return Err(anyhow!("Failed to get stream info: {}", response.status));
        }

        let mut content_length = 0;
        let mut content_type = "audio/mpeg".to_string();

        for (name, value) in &response.headers {
            let lower_name = name.to_lowercase();
            if lower_name == "content-length" {
                if let Ok(len) = value.parse::<u64>() {
                    content_length = len;
                }
            } else if lower_name == "content-type" {
                content_type = value.clone();
            }
        }

        Ok(StreamInfo {
            url: url.to_string(),
            content_length,
            content_type,
        })
    }

    #[allow(dead_code)]
    pub fn download_range(&self, url: &str, start: u64, end: u64) -> Result<Vec<u8>> {
        let options = RequestOptions {
            method: HttpMethod::Get,
            headers: Some(vec![
                ("Range".to_string(), format!("bytes={}-{}", start, end)),
                (
                    "User-Agent".to_string(),
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                ),
            ]),
            body: None,
            timeout_seconds: Some(30),
        };

        let response = http_request(url, &options).map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if response.status != 200 && response.status != 206 {
            return Err(anyhow!("Range request failed: {}", response.status));
        }

        Ok(response.body)
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
#[allow(dead_code)]
pub fn parse_quality_from_url(url: &str) -> Option<String> {
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
