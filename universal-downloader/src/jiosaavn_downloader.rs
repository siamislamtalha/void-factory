//! JioSaavn download methods with DES-ECB decryption
//!
//! This module implements JioSaavn download using:
//! - DES-ECB decryption for encrypted stream URLs
//! - Multiple server endpoints with automatic rotation
//! - Quality selection (96kbps, 128kbps, 160kbps, 320kbps)

use crate::credentials::{JIOSAAVN_CONFIG, JIOSAAVN_DES_KEY, JIOSAAVN_SERVERS};
use anyhow::{anyhow, Result};
use des::cipher::BlockCipher;
use des::Des;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str;

#[derive(Debug, Serialize, Deserialize)]
struct JioSaavnTrackRequest {
    #[serde(rename = "ids")]
    ids: String,
    #[serde(rename = "type")]
    track_type: String,
}

#[derive(Debug, Deserialize)]
struct JioSaavnTrackResponse {
    #[serde(rename = "songs")]
    songs: Option<Vec<JioSaavnSong>>,
}

#[derive(Debug, Deserialize)]
struct JioSaavnSong {
    #[serde(rename = "id")]
    id: String,
    #[serde(rename = "media_preview_url")]
    media_preview_url: Option<String>,
    #[serde(rename = "encrypted_media_url")]
    encrypted_media_url: Option<String>,
    #[serde(rename = "album")]
    album: Option<String>,
    #[serde(rename = "artist")]
    artist: Option<String>,
    #[serde(rename = "title")]
    title: Option<String>,
    #[serde(rename = "image")]
    image: Option<String>,
    #[serde(rename = "duration")]
    duration: Option<String>,
}

pub struct JioSaavnDownloader {
    client: Client,
    server: String,
}

impl JioSaavnDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            server: JIOSAAVN_SERVERS.get_current(),
        }
    }

    pub fn with_rotated_server() -> Self {
        Self {
            client: Client::new(),
            server: JIOSAAVN_SERVERS.rotate(),
        }
    }

    pub async fn get_stream_url(&self, track_id: &str, quality: JioSaavnQuality) -> Result<String> {
        let url = format!(
            "https://{}/api?__call=webapi.get&token={}&type=song&includeMetaTags=0&ctx=web6dot0&api_version=4&_format=json&_marker=0",
            self.server, track_id
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        let text = response.text().await?;
        
        // Parse the JSON response (JioSaavn sometimes wraps in parentheses)
        let json_str = text.trim_start_matches('(').trim_end_matches(')');
        let track_response: JioSaavnTrackResponse = serde_json::from_str(json_str)?;

        let song = track_response
            .songs
            .and_then(|songs| songs.into_iter().next())
            .ok_or_else(|| anyhow!("No song found in response"))?;

        let encrypted_url = song
            .encrypted_media_url
            .or(song.media_preview_url)
            .ok_or_else(|| anyhow!("No media URL available"))?;

        let decrypted_url = self.decrypt_url(&encrypted_url)?;
        
        // Apply quality selection
        let quality_url = self.apply_quality(&decrypted_url, quality)?;
        
        Ok(quality_url)
    }

    fn decrypt_url(&self, encrypted_url: &str) -> Result<String> {
        // JioSaavn uses DES-ECB decryption
        let cipher = Des::new(JIOSAAVN_DES_KEY);
        
        // Decode base64
        let encrypted_bytes = base64::decode(encrypted_url)?;
        
        // DES-ECB requires 8-byte blocks
        let block_size = 8;
        if encrypted_bytes.len() % block_size != 0 {
            return Err(anyhow!("Encrypted data length not divisible by block size"));
        }
        
        let mut decrypted_bytes = Vec::new();
        for chunk in encrypted_bytes.chunks(block_size) {
            let mut block = [0u8; 8];
            block.copy_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            decrypted_bytes.extend_from_slice(&block);
        }
        
        // Remove PKCS7 padding
        if let Some(&padding_len) = decrypted_bytes.last() {
            if padding_len as usize <= block_size && padding_len > 0 {
                let padding_start = decrypted_bytes.len().saturating_sub(padding_len as usize);
                if decrypted_bytes[padding_start..].iter().all(|&b| b == padding_len) {
                    decrypted_bytes.truncate(padding_start);
                }
            }
        }
        
        let decrypted_str = str::from_utf8(&decrypted_bytes)?;
        Ok(decrypted_str.to_string())
    }

    fn apply_quality(&self, url: &str, quality: JioSaavnQuality) -> Result<String> {
        // JioSaavn uses _96, _128, _160, _320 suffixes for quality
        let base_url = url.replace("_96.", ".").replace("_128.", ".").replace("_160.", ".").replace("_320.", ".");
        
        let quality_suffix = match quality {
            JioSaavnQuality::Low => "_96",
            JioSaavnQuality::Medium => "_128",
            JioSaavnQuality::High => "_160",
            JioSaavnQuality::VeryHigh => "_320",
        };
        
        // Insert quality suffix before the file extension
        if let Some(dot_pos) = base_url.rfind('.') {
            let (before_ext, ext) = base_url.split_at(dot_pos);
            Ok(format!("{}{}{}", before_ext, quality_suffix, ext))
        } else {
            Ok(format!("{}{}", base_url, quality_suffix))
        }
    }
}

impl Default for JioSaavnDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JioSaavnQuality {
    Low,      // 96 kbps
    Medium,   // 128 kbps
    High,     // 160 kbps
    VeryHigh, // 320 kbps
}

impl Default for JioSaavnQuality {
    fn default() -> Self {
        Self::High
    }
}
