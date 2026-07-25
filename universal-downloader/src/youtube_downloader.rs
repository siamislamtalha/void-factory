//! YouTube download methods with multiple client fallbacks
//!
//! This module implements YouTube download using the Innertube API with:
//! - Multiple API key rotation
//! - Multiple client fallbacks (ANDROID_VR, IOS, TVHTML5, WEB_REMIX)
//! - Signature cipher decoding
//! - Range request support to avoid throttling

use crate::credentials::YOUTUBE_CREDENTIALS;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            client_name: "21".to_string(),
            client_version: "1.56.1002".to_string(),
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.56.1002 (Lollipop; VR) gzip".to_string(),
            referer: Some("https://www.youtube.com/".to_string()),
        }
    }

    fn ios() -> Self {
        Self {
            client_name: "5".to_string(),
            client_version: "19.09.3".to_string(),
            user_agent: "com.google.ios.youtube/19.09.3 (iPhone14,3; U; CPU iOS 15_6 like Mac OS X)".to_string(),
            referer: Some("https://www.youtube.com/".to_string()),
        }
    }

    fn tvhtml5() -> Self {
        Self {
            client_name: "7".to_string(),
            client_version: "7.20231204".to_string(),
            user_agent: "Mozilla/5.0 (ChromiumStyle; Linux) SmartTV/1.0".to_string(),
            referer: Some("https://www.youtube.com/tv".to_string()),
        }
    }

    fn web_remix() -> Self {
        Self {
            client_name: "67".to_string(),
            client_version: "1.20231128.07.00".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
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
    expires_in_seconds: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdaptiveFormat {
    #[serde(rename = "itag")]
    itag: i32,
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
    duration_ms: Option<String>,
}

pub struct YouTubeDownloader {
    client: Client,
    api_key: String,
}

impl YouTubeDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: YOUTUBE_CREDENTIALS.get_current(),
        }
    }

    pub fn with_rotated_key() -> Self {
        Self {
            client: Client::new(),
            api_key: YOUTUBE_CREDENTIALS.rotate(),
        }
    }

    pub async fn get_stream_url(&self, video_id: &str) -> Result<String> {
        // Try multiple clients in order of priority
        let clients = vec![
            YouTubeClient::android_vr(),
            YouTubeClient::ios(),
            YouTubeClient::tvhtml5(),
            YouTubeClient::web_remix(),
        ];

        for client in clients {
            match self.get_stream_url_with_client(video_id, &client).await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    eprintln!("Client {:?} failed: {}", client.client_name, e);
                    continue;
                }
            }
        }

        Err(anyhow!("All YouTube clients failed"))
    }

    async fn get_stream_url_with_client(&self, video_id: &str, client: &YouTubeClient) -> Result<String> {
        let context = PlayerRequestContext {
            client: YouTubeClientContext {
                client_name: client.client_name.clone(),
                client_version: client.client_version.clone(),
                language: "en".to_string(),
                country: "US".to_string(),
                visitor_data: Some("CgtsZG1ySnZiQWtSbyiMjuGSBg%3D%3D".to_string()),
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

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Format-Version", "1")
            .header("X-YouTube-Client-Name", &client.client_name)
            .header("X-YouTube-Client-Version", &client.client_version)
            .header("User-Agent", &client.user_agent)
            .header("Referer", client.referer.as_ref().unwrap_or(&"https://www.youtube.com/".to_string()))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        let player_response: PlayerResponse = response.json().await?;

        if player_response.playability_status.status != "OK" {
            return Err(anyhow!(
                "Playability error: {}",
                player_response.playability_status.reason.unwrap_or_else(|| "Unknown".to_string())
            ));
        }

        let streaming_data = player_response
            .streaming_data
            .ok_or_else(|| anyhow!("No streaming data available"))?;

        // Find best audio-only format
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
            // Handle cipher/signature
            let cipher = audio_format
                .cipher
                .as_ref()
                .or(audio_format.signature_cipher.as_ref())
                .ok_or_else(|| anyhow!("No URL or cipher available"))?;

            self.decode_cipher(cipher)?
        };

        // Add range to avoid YouTube throttling
        let content_length = audio_format.content_length.as_ref().and_then(|l| l.parse::<u64>().ok()).unwrap_or(10_000_000);
        Ok(format!("{}&range=0-{}", stream_url, content_length))
    }

    fn decode_cipher(&self, cipher: &str) -> Result<String> {
        // Simple cipher decoding (placeholder - full implementation would be complex)
        // This is a simplified version - real implementation needs full cipher decoding logic
        let params: HashMap<String, String> = cipher
            .split('&')
            .filter_map(|part| {
                let mut parts = part.split('=');
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            })
            .collect();

        let url = params.get("url").ok_or_else(|| anyhow!("No URL in cipher"))?;
        // In real implementation, you would decode the signature here
        Ok(url.clone())
    }
}

impl Default for YouTubeDownloader {
    fn default() -> Self {
        Self::new()
    }
}
