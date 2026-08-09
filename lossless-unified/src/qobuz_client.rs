use crate::types::*;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use md5::{Md5, Digest};
use serde_json::Value;
use lazy_static::lazy_static;
use regex::Regex;
use base64::{Engine as _, engine::general_purpose};
use std::collections::BTreeMap;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

const QOBUZ_BASE_URL: &str = "https://www.qobuz.com/api.json/0.2";
const QOBUZ_LOGIN_URL: &str = "https://play.qobuz.com/login";

// Quality mapping from streamrip (format IDs)
const QUALITY_MAP: &[u32] = &[5, 6, 7, 27]; // 320kbps, FLAC, 24-bit <=96kHz, 24-bit >96kHz

/// Qobuz client for high-quality FLAC downloads
pub struct QobuzClient {
    app_id: String,
    secret: String,
    user_auth_token: Option<String>,
}

impl QobuzClient {
    pub fn new(app_id: String, secret: String) -> Self {
        Self {
            app_id,
            secret,
            user_auth_token: None,
        }
    }

    pub fn with_auth(mut self, user_auth_token: String) -> Self {
        self.user_auth_token = Some(user_auth_token);
        self
    }

    /// Create a new Qobuz client with automatic credential fetching
    pub async fn new_auto() -> Result<Self, String> {
        match Self::fetch_app_id_and_secrets().await {
            Ok((app_id, secrets)) => {
                let secret = secrets.first()
                    .ok_or("No valid secrets found")?
                    .clone();
                Ok(Self::new(app_id, secret))
            }
            Err(e) => {
                eprintln!("Dynamic Qobuz credential extraction failed: {}. Using fallback.", e);
                Self::fetch_app_id_and_secrets_fallback().await.map(|(app_id, secrets)| {
                    let secret = secrets.first().unwrap_or(&String::new()).clone();
                    Self::new(app_id, secret)
                })
            }
        }
    }

    /// Fetch app_id and secrets from Qobuz's bundle.js (streamrip implementation)
    async fn fetch_app_id_and_secrets() -> Result<(String, Vec<String>), String> {
        // From streamrip qobuz.py - fetch from play.qobuz.com/login
        let login_page = HTTP_CLIENT
            .get(QOBUZ_LOGIN_URL)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch login page: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read login page: {}", e))?;

        // Extract bundle URL using streamrip regex
        let bundle_url_regex = Regex::new(r#"<script src="(/resources/\d+\.\d+\.\d+-[a-z]\d{3}/bundle\.js)"></script>"#)
            .map_err(|e| format!("Failed to create bundle URL regex: {}", e))?;
        
        let bundle_url = bundle_url_regex
            .captures(&login_page)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or("Could not find bundle URL")?;

        let full_bundle_url = format!("https://play.qobuz.com{}", bundle_url);

        // Fetch bundle
        let bundle = HTTP_CLIENT
            .get(&full_bundle_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch bundle: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read bundle: {}", e))?;

        // Extract app_id using streamrip regex pattern
        let app_id_regex = Regex::new(r#"production:\{api:\{appId:"(\d{9})",appSecret:"(\w{32})""#)
            .map_err(|e| format!("Failed to create app_id regex: {}", e))?;
        
        let app_id = app_id_regex
            .captures(&bundle)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or("Could not find app_id")?
            .to_string();

        // Extract seeds using streamrip pattern
        let seed_regex = Regex::new(r#"[a-z]\.initialSeed\("([\w=]+)",window\.utimezone\.([a-z]+)\)"#)
            .map_err(|e| format!("Failed to create seed regex: {}", e))?;
        
        let mut secrets: BTreeMap<String, Vec<String>> = BTreeMap::new();
        
        for cap in seed_regex.captures_iter(&bundle) {
            let seed = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let timezone = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            secrets.entry(timezone.to_lowercase()).or_insert_with(Vec::new).push(seed.to_string());
        }

        // Reorder keys (move second to first as per streamrip logic)
        // This is because JavaScript ternary operators prioritize the second option
        let keys: Vec<String> = secrets.keys().cloned().collect();
        if keys.len() >= 2 {
            let first = keys[0].clone();
            let second = keys[1].clone();
            secrets.remove(&first);
            secrets.remove(&second);
            secrets.insert(second, vec![]);
            secrets.insert(first, vec![]);
        }

        // Extract info_extras using streamrip pattern
        let timezones: String = secrets.keys().cloned().collect::<Vec<_>>().join("|");
        let info_extras_regex = Regex::new(&format!(
            r#"name:"\w+/({})",info:"([\w=]+)",extras:"([\w=]+)""#,
            timezones
        )).map_err(|e| format!("Failed to create info_extras regex: {}", e))?;
        
        for cap in info_extras_regex.captures_iter(&bundle) {
            let timezone = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let info = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let extras = cap.get(3).map(|m| m.as_str()).unwrap_or("");
            
            if let Some(entry) = secrets.get_mut(&timezone.to_lowercase()) {
                entry.push(info.to_string());
                entry.push(extras.to_string());
            }
        }

        // Decode secrets using streamrip logic
        let mut decoded_secrets = Vec::new();
        
        for (timezone, parts) in &secrets {
            if parts.len() >= 3 {
                let combined = format!("{}{}{}", parts[0], parts[1], parts[2]);
                if combined.len() > 44 {
                    let truncated = &combined[..combined.len() - 44];
                    match general_purpose::STANDARD.decode(truncated) {
                        Ok(decoded) => {
                            if let Ok(decoded_str) = String::from_utf8(decoded) {
                                if !decoded_str.is_empty() {
                                    decoded_secrets.push(decoded_str);
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        // Test secrets against a known track ID (streamrip uses track 19512574)
        let test_track_id = "19512574";
        let valid_secret = Self::find_valid_secret(&app_id, &decoded_secrets, test_track_id).await?;

        Ok((app_id, vec![valid_secret]))
    }
    
    /// Fallback to hardcoded app_id if dynamic extraction fails
    async fn fetch_app_id_and_secrets_fallback() -> Result<(String, Vec<String>), String> {
        // Use known working app_id and secret from streamrip config
        let app_id = "639242930".to_string();
        let secret = "OJzm0Xr8rCb8sT8S"; // Known working secret
        
        Ok((app_id, vec![secret]))
    }

    /// Find valid secret by testing against a track
    async fn find_valid_secret(app_id: &str, secrets: &[String], track_id: &str) -> Result<String, String> {
        for secret in secrets {
            if Self::test_secret(app_id, secret, track_id).await {
                return Ok(secret.clone());
            }
        }
        Err("No valid secret found".to_string())
    }

    /// Test if a secret is valid
    async fn test_secret(app_id: &str, secret: &str, track_id: &str) -> bool {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let signature_str = format!(
            "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
            6, track_id, timestamp, secret
        );

        let mut hasher = Md5::new();
        hasher.update(signature_str.as_bytes());
        let signature = hex::encode(hasher.finalize());

        let params = [
            ("track_id", track_id),
            ("format_id", "6"),
            ("intent", "stream"),
            ("request_ts", &timestamp.to_string()),
            ("request_sig", &signature),
        ];

        let response = HTTP_CLIENT
            .get(&format!("{}/track/getFileUrl", QOBUZ_BASE_URL))
            .header("X-App-Id", app_id)
            .query(&params)
            .send()
            .await;

        match response {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Search for tracks on Qobuz
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
        });

        let response = self.api_request("track/search", &params).await?;
        
        if let Some(tracks) = response.get("tracks").and_then(|t| t.get("items")) {
            if let Some(items) = tracks.as_array() {
                return items.iter()
                    .filter_map(|item| self.parse_track(item))
                    .collect();
            }
        }
        
        Ok(vec![])
    }

    /// Search for albums on Qobuz
    pub async fn search_albums(&self, query: &str, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
        });

        let response = self.api_request("album/search", &params).await?;
        
        if let Some(albums) = response.get("albums").and_then(|a| a.get("items")) {
            if let Some(items) = albums.as_array() {
                return items.iter()
                    .filter_map(|item| self.parse_album(item))
                    .collect();
            }
        }
        
        Ok(vec![])
    }

    /// Search for artists on Qobuz
    pub async fn search_artists(&self, query: &str, limit: u32) -> Result<Vec<UnifiedArtist>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
        });

        let response = self.api_request("artist/search", &params).await?;
        
        if let Some(artists) = response.get("artists").and_then(|a| a.get("items")) {
            if let Some(items) = artists.as_array() {
                return items.iter()
                    .filter_map(|item| self.parse_artist(item))
                    .collect();
            }
        }
        
        Ok(vec![])
    }

    /// Get track details
    pub async fn get_track(&self, track_id: &str) -> Result<UnifiedTrack, String> {
        let params = serde_json::json!({
            "track_id": track_id,
        });

        let response = self.api_request("track/get", &params).await?;
        self.parse_track(&response).ok_or("Failed to parse track".to_string())
    }

    /// Get album details
    pub async fn get_album(&self, album_id: &str) -> Result<UnifiedAlbum, String> {
        let params = serde_json::json!({
            "album_id": album_id,
            "extra": "tracks",
        });

        let response = self.api_request("album/get", &params).await?;
        self.parse_album(&response).ok_or("Failed to parse album".to_string())
    }

    /// Get stream URL for a track at specified quality
    pub async fn get_stream_url(&self, track_id: &str, quality: u8) -> Result<StreamInfo, String> {
        let format_id = self.quality_to_format_id(quality);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let signature_str = format!(
            "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
            format_id, track_id, timestamp, self.secret
        );

        let mut hasher = Md5::new();
        hasher.update(signature_str.as_bytes());
        let signature = hex::encode(hasher.finalize());

        let params = serde_json::json!({
            "track_id": track_id,
            "format_id": format_id,
            "intent": "stream",
            "request_ts": timestamp,
            "request_sig": signature,
        });

        let response = self.api_request("track/getFileUrl", &params).await?;
        
        let url = response.get("url")
            .and_then(|u| u.as_str())
            .ok_or("No stream URL found")?;

        let (sample_rate, bit_depth) = self.quality_to_sample_params(quality);

        Ok(StreamInfo {
            track_id: track_id.to_string(),
            quality: Quality::from_number(quality.min(4)),
            codec: if quality > 1 { "flac".to_string() } else { "mp3".to_string() },
            url: url.to_string(),
            encryption_key: None,
            source: MusicSource::Qobuz,
            bitrate: Quality::from_number(quality.min(4)).bitrate(),
            sample_rate,
            bit_depth,
        })
    }

    /// Get featured/new releases
    pub async fn get_featured(&self, feature_type: &str, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "type": feature_type,
            "limit": limit,
        });

        let response = self.api_request("album/getFeatured", &params).await?;
        
        if let Some(albums) = response.get("albums").and_then(|a| a.get("items")) {
            if let Some(items) = albums.as_array() {
                return items.iter()
                    .filter_map(|item| self.parse_album(item))
                    .collect();
            }
        }
        
        Ok(vec![])
    }

    /// Get featured albums (alias for get_featured)
    pub async fn get_featured_albums(&self, feature_type: &str, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        self.get_featured(feature_type, limit).await
    }

    async fn api_request(&self, endpoint: &str, params: &Value) -> Result<Value, String> {
        let url = format!("{}/{}", QOBUZ_BASE_URL, endpoint);
        
        let mut request = HTTP_CLIENT.get(&url);
        
        if let Some(token) = &self.user_auth_token {
            request = request.header("X-User-Auth-Token", token);
        }
        
        request = request.header("X-App-Id", &self.app_id);
        request = request.query(params);

        let response = request.send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    fn parse_track(&self, item: &Value) -> Option<UnifiedTrack> {
        let id = item.get("id")?.as_str()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        let duration = item.get("duration")?.as_u64()? as u32;
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        let album = item.get("album").and_then(|a| self.parse_album(a));
        
        // Determine available qualities
        let qualities_available = vec![Quality::LosslessFlac, Quality::HiRes, Quality::UltraHiRes];

        Some(UnifiedTrack {
            id,
            title,
            duration,
            track_number: item.get("track_number").and_then(|n| n.as_u64()).map(|n| n as u32),
            volume_number: item.get("volume_number").and_then(|n| n.as_u64()).map(|n| n as u32),
            replay_gain: None,
            peak: None,
            available: true,
            audio_quality: Some("LOSSLESS".to_string()),
            audio_modes: None,
            artist,
            artists: None,
            album,
            source: MusicSource::Qobuz,
            qualities_available,
        })
    }

    fn parse_album(&self, item: &Value) -> Option<UnifiedAlbum> {
        let id = item.get("id")?.as_str()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        
        Some(UnifiedAlbum {
            id,
            title,
            cover: item.get("image").and_then(|i| i.as_str()).map(|s| s.to_string()),
            duration: item.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32),
            track_count: item.get("tracks_count").and_then(|c| c.as_u64()).map(|c| c as u32),
            release_date: item.get("release_date").and_then(|d| d.as_str()).map(|s| s.to_string()),
            artist,
            artists: None,
            url: item.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::Qobuz,
        })
    }

    fn parse_artist(&self, item: &Value) -> Option<UnifiedArtist> {
        Some(UnifiedArtist {
            id: item.get("id")?.as_str()?.to_string(),
            name: item.get("name")?.as_str()?.to_string(),
            picture: item.get("image").and_then(|i| i.as_str()).map(|s| s.to_string()),
            url: item.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::Qobuz,
        })
    }

    fn quality_to_format_id(&self, quality: u8) -> u32 {
        // Qobuz format IDs from streamrip: 5=MP3_128, 6=MP3_320, 7=FLAC, 27=FLAC_HIRES
        match quality {
            0 => 5,   // MP3 128kbps
            1 => 6,   // MP3 320kbps
            2 => 7,   // FLAC 16-bit/44.1kHz
            3 => 27,  // FLAC 24-bit <=96kHz
            4 => 27,  // FLAC 24-bit >96kHz (uses same format ID as 3)
            _ => 7,   // Default to FLAC
        }
    }

    fn quality_to_sample_params(&self, quality: u8) -> (u32, u8) {
        match quality {
            0 => (44100, 16),
            1 => (44100, 16),
            2 => (44100, 16),
            3 => (96000, 24),
            4 => (192000, 24),
            _ => (44100, 16),
        }
    }
}
