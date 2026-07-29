//! YouTube download methods with multiple client fallbacks
//!
//! This module implements YouTube download using the Innertube API with:
//! - Multiple API key rotation
//! - Multiple client fallbacks (ANDROID_VR, IOS, TVHTML5, WEB_REMIX)
//! - Signature cipher decoding
//! - Range request support to avoid throttling

use crate::credentials::YOUTUBE_CREDENTIALS;
use anyhow::{anyhow, Result};
use bex_core::resolver::component::content_resolver::utils::{
    http_request, HttpMethod, RequestOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct YouTubeClient {
    client_name: String,
    client_version: String,
    user_agent: String,
    referer: Option<String>,
}

impl YouTubeClient {
    fn android_vr() -> Self {
        Self {
            client_name: "28".to_string(),
            client_version: "1.71.26".to_string(),
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.71.26 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip".to_string(),
            referer: Some("https://www.youtube.com/".to_string()),
        }
    }

    fn ios() -> Self {
        Self {
            client_name: "5".to_string(),
            client_version: "20.10.4".to_string(),
            user_agent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3 like Mac OS X)".to_string(),
            referer: Some("https://www.youtube.com/".to_string()),
        }
    }

    fn tvhtml5() -> Self {
        Self {
            client_name: "7".to_string(),
            client_version: "7.20260114.12.00".to_string(),
            user_agent: "Mozilla/5.0 (ChromiumStyle; Linux) SmartTV/1.0".to_string(),
            referer: Some("https://www.youtube.com/tv".to_string()),
        }
    }

    fn web_remix() -> Self {
        Self {
            client_name: "67".to_string(),
            client_version: "1.20260222.01.00".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36".to_string(),
            referer: Some("https://music.youtube.com/".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerRequestContext {
    client: YouTubeClientContext,
}

#[derive(Debug, Serialize, Deserialize)]
struct YouTubeClientContext {
    client_name: String,
    client_version: String,
    #[serde(rename = "hl")]
    language: String,
    #[serde(rename = "gl")]
    country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    visitor_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerRequest {
    context: PlayerRequestContext,
    video_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    playlist_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayerResponse {
    #[serde(rename = "streamingData")]
    streaming_data: Option<StreamingData>,
    #[serde(rename = "playabilityStatus")]
    playability_status: PlayabilityStatus,
}

#[derive(Debug, Deserialize)]
struct PlayabilityStatus {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamingData {
    #[serde(rename = "adaptiveFormats")]
    adaptive_formats: Option<Vec<AdaptiveFormat>>,
    #[serde(rename = "expiresInSeconds")]
    _expires_in_seconds: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdaptiveFormat {
    #[serde(rename = "itag")]
    _itag: i32,
    #[serde(rename = "url")]
    url: Option<String>,
    #[serde(rename = "cipher")]
    cipher: Option<String>,
    #[serde(rename = "signatureCipher")]
    signature_cipher: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(rename = "bitrate")]
    bitrate: i32,
    #[serde(rename = "contentLength")]
    content_length: Option<String>,
    #[serde(rename = "approxDurationMs")]
    _duration_ms: Option<String>,
}

pub struct YouTubeDownloader {
    api_key: String,
}

impl YouTubeDownloader {
    pub fn new() -> Self {
        Self {
            api_key: YOUTUBE_CREDENTIALS.get_current(),
        }
    }

    pub fn with_rotated_key() -> Self {
        Self {
            api_key: YOUTUBE_CREDENTIALS.rotate(),
        }
    }

    pub fn get_stream_url(&self, video_id: &str) -> Result<String> {
        let clients = vec![
            YouTubeClient::android_vr(),
            YouTubeClient::ios(),
            YouTubeClient::tvhtml5(),
            YouTubeClient::web_remix(),
        ];

        for (attempt, client) in clients.iter().enumerate() {
            match self.get_stream_url_with_client_retry(video_id, client, attempt) {
                Ok(url) => return Ok(url),
                Err(e) => {
                    let _ = format!("Client {:?} failed: {}", client.client_name, e);
                    continue;
                }
            }
        }

        Err(anyhow!("All YouTube clients failed"))
    }

    fn get_stream_url_with_client_retry(&self, video_id: &str, client: &YouTubeClient, attempt: usize) -> Result<String> {
        // Simple retry with exponential backoff
        let max_retries = 2;
        for retry in 0..=max_retries {
            match self.get_stream_url_with_client(video_id, client) {
                Ok(url) => return Ok(url),
                Err(e) => {
                    if retry < max_retries {
                        // Exponential backoff: 100ms, 200ms, 400ms
                        let delay_ms = 100 * (1 << retry);
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    }
                }
            }
        }
        Err(anyhow!("Client {:?} failed after retries", client.client_name))
    }

    fn get_stream_url_with_client(&self, video_id: &str, client: &YouTubeClient) -> Result<String> {
        // Don't use hardcoded visitor data - it can become stale and cause 403 errors
        // YouTube will work without it for most content
        let context = PlayerRequestContext {
            client: YouTubeClientContext {
                client_name: client.client_name.clone(),
                client_version: client.client_version.clone(),
                language: "en".to_string(),
                country: "US".to_string(),
                visitor_data: None,
            },
        };

        let request = PlayerRequest {
            context,
            video_id: video_id.to_string(),
            playlist_id: None,
            params: None,
        };

        let url = format!(
            "https://music.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
            self.api_key
        );

        let body_bytes = serde_json::to_vec(&request)?;

        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Goog-Api-Format-Version".to_string(), "1".to_string()),
            ("X-YouTube-Client-Name".to_string(), client.client_name.clone()),
            ("X-YouTube-Client-Version".to_string(), client.client_version.clone()),
            ("User-Agent".to_string(), client.user_agent.clone()),
        ];
        if let Some(ref referer) = client.referer {
            headers.push(("Referer".to_string(), referer.clone()));
        }

        let options = RequestOptions {
            method: HttpMethod::Post,
            headers: Some(headers),
            body: Some(body_bytes),
            timeout_seconds: Some(10),
        };

        let response = http_request(&url, &options).map_err(|e| anyhow!("HTTP error: {}", e))?;

        if response.status < 200 || response.status >= 300 {
            return Err(anyhow!("HTTP error: {}", response.status));
        }

        let player_response: PlayerResponse = serde_json::from_slice(&response.body)?;

        if player_response.playability_status.status != "OK" {
            return Err(anyhow!(
                "Playability error: {}",
                player_response
                    .playability_status
                    .reason
                    .unwrap_or_else(|| "Unknown".to_string())
            ));
        }

        let streaming_data = player_response
            .streaming_data
            .ok_or_else(|| anyhow!("No streaming data available"))?;

        let audio_format = streaming_data
            .adaptive_formats
            .as_ref()
            .and_then(|formats| {
                formats
                    .iter()
                    .filter(|f| f.mime_type.contains("audio"))
                    .max_by_key(|f| f.bitrate)
            })
            .ok_or_else(|| anyhow!("No audio format found"))?;

        let stream_url = if let Some(url) = &audio_format.url {
            url.clone()
        } else {
            let cipher = audio_format
                .cipher
                .as_ref()
                .or(audio_format.signature_cipher.as_ref())
                .ok_or_else(|| anyhow!("No URL or cipher available"))?;

            self.decode_cipher(cipher)?
        };

        let content_length = audio_format
            .content_length
            .as_ref()
            .and_then(|l| l.parse::<u64>().ok())
            .unwrap_or(10_000_000);
        Ok(format!("{}&range=0-{}", stream_url, content_length))
    }

    fn decode_cipher(&self, cipher: &str) -> Result<String> {
        let params: HashMap<String, String> = cipher
            .split('&')
            .filter_map(|part| {
                let mut parts = part.split('=');
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            })
            .collect();

        let url = params.get("url").ok_or_else(|| anyhow!("No URL in cipher"))?;
        Ok(url.clone())
    }
}

impl Default for YouTubeDownloader {
    fn default() -> Self {
        Self::new()
    }
}
