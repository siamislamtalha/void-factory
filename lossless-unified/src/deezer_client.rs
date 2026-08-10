use crate::types::*;
use crate::decryption::{decrypt_blowfish, decrypt_aes, generate_blowfish_key};
use reqwest::Client;
use serde_json::Value;
use lazy_static::lazy_static;
use std::collections::HashMap;
use md5::{Md5, Digest};

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

const DEEZER_BASE_URL: &str = "https://api.deezer.com";
const DEEZER_GW_URL: &str = "https://www.deezer.com/ajax/gw-light.php";

const BLOWFISH_SECRET: &str = "g4el58wc0zvf9na1";
const AES_KEY: &str = "jo6aey6haid2Teih";

/// Deezer client for FLAC streaming
pub struct DeezerClient {
    arl: String,
    session_id: String,
}

impl DeezerClient {
    pub fn new(arl: String) -> Self {
        Self {
            arl,
            session_id: Self::generate_session_id(),
        }
    }

    /// Search for tracks on Deezer
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let url = format!("{}/search/track", DEEZER_BASE_URL);
        
        let params = [
            ("q", query),
            ("limit", &limit.to_string()),
            ("output", "json"),
        ];

        let response = HTTP_CLIENT
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
            let parsed: Vec<UnifiedTrack> = items.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
            return Ok(parsed);
        }
        
        Ok(vec![])
    }

    /// Search for albums on Deezer
    pub async fn search_albums(&self, query: &str, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let url = format!("{}/search/album", DEEZER_BASE_URL);
        
        let params = [
            ("q", query),
            ("limit", &limit.to_string()),
            ("output", "json"),
        ];

        let response = HTTP_CLIENT
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
            let parsed: Vec<UnifiedAlbum> = items.iter()
                .filter_map(|item| self.parse_album(item))
                .collect();
            return Ok(parsed);
        }
        
        Ok(vec![])
    }

    /// Search for artists on Deezer
    pub async fn search_artists(&self, query: &str, limit: u32) -> Result<Vec<UnifiedArtist>, String> {
        let url = format!("{}/search/artist", DEEZER_BASE_URL);
        
        let params = [
            ("q", query),
            ("limit", &limit.to_string()),
            ("output", "json"),
        ];

        let response = HTTP_CLIENT
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
            let parsed: Vec<UnifiedArtist> = items.iter()
                .filter_map(|item| self.parse_artist(item))
                .collect();
            return Ok(parsed);
        }
        
        Ok(vec![])
    }

    /// Get track details
    pub async fn get_track(&self, track_id: &str) -> Result<UnifiedTrack, String> {
        let url = format!("{}/track/{}", DEEZER_BASE_URL, track_id);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("output", "json")])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        self.parse_track(&data).ok_or("Failed to parse track".to_string())
    }

    /// Get album details
    pub async fn get_album(&self, album_id: &str) -> Result<UnifiedAlbum, String> {
        let url = format!("{}/album/{}", DEEZER_BASE_URL, album_id);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("output", "json")])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        self.parse_album(&data).ok_or("Failed to parse album".to_string())
    }

    /// Get stream URL for a track at specified quality
    pub async fn get_stream_url(&self, track_id: &str, quality: u8) -> Result<StreamInfo, String> {
        // Deezer uses a different API for streaming
        // We need to get track info from the GW API
        let gw_url = DEEZER_GW_URL;
        
        let mut params = HashMap::new();
        params.insert("method", "song.getTrackUrl");
        params.insert("sng_id", track_id);
        params.insert("quality", &self.quality_to_deezer_format(quality).to_string());
        params.insert("nb", "0");
        params.insert("api_token", "null");
        params.insert("output", "json");

        let response = HTTP_CLIENT
            .post(gw_url)
            .header("Cookie", format!("arl={}", self.arl))
            .header("User-Agent", "Mozilla/5.0")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let url = data.get("data")
            .and_then(|d| d.as_str())
            .ok_or("No stream URL found")?;

        // Check if URL is encrypted (contains /mobile/ or /media/)
        let is_encrypted = url.contains("/mobile/") || url.contains("/media/");
        
        let encryption_key = if is_encrypted {
            // Generate blowfish key for decryption
            let blowfish_key = generate_blowfish_key(track_id);
            Some(hex::encode(blowfish_key))
        } else {
            None
        };

        let (sample_rate, bit_depth) = self.quality_to_sample_params(quality);

        Ok(StreamInfo {
            track_id: track_id.to_string(),
            quality: Quality::from_number(quality.min(2)),
            codec: if quality > 1 { "flac".to_string() } else { "mp3".to_string() },
            url: url.to_string(),
            encryption_key,
            source: MusicSource::Deezer,
            bitrate: Quality::from_number(quality.min(2)).bitrate(),
            sample_rate,
            bit_depth,
        })
    }

    /// Get featured/new releases
    pub async fn get_featured(&self, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let url = format!("{}/editorial/0/charts", DEEZER_BASE_URL);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("limit", &limit.to_string()), ("output", &String::from("json"))])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Parse albums from charts response
        if let Some(albums) = data.get("albums").and_then(|a| a.get("data")) {
            if let Some(items) = albums.as_array() {
                let parsed: Vec<UnifiedAlbum> = items.iter()
                    .filter_map(|item| self.parse_album(item))
                    .collect();
                return Ok(parsed);
            }
        }

        Ok(vec![])
    }

    /// Get charts (alias for get_featured)
    pub async fn get_charts(&self, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let url = format!("{}/editorial/0/charts", DEEZER_BASE_URL);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("limit", &limit.to_string()), ("output", &String::from("json"))])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Parse tracks from charts response
        if let Some(tracks) = data.get("tracks").and_then(|t| t.get("data")) {
            if let Some(items) = tracks.as_array() {
                let parsed: Vec<UnifiedTrack> = items.iter()
                    .filter_map(|item| self.parse_track(item))
                    .collect();
                return Ok(parsed);
            }
        }

        Ok(vec![])
    }

    /// Get new releases
    pub async fn get_new_releases(&self, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let url = format!("{}/editorial/0/releases", DEEZER_BASE_URL);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("limit", &limit.to_string()), ("output", &String::from("json"))])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(albums) = data.get("albums").and_then(|a| a.get("data")) {
            if let Some(items) = albums.as_array() {
                return items.iter()
                    .filter_map(|item| self.parse_album(item))
                    .collect();
            }
        }
        
        Ok(vec![])
    }

    fn generate_session_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let session_id: u64 = rng.gen();
        format!("{}", session_id)
    }

    fn quality_to_deezer_format(&self, quality: u8) -> u32 {
        // Deezer quality: 1=MP3_128, 3=MP3_320, 9=FLAC
        match quality {
            0 => 1,
            1 => 3,
            2 => 9,
            _ => 9,
        }
    }

    fn quality_to_sample_params(&self, quality: u8) -> (u32, u8) {
        match quality {
            0 => (44100, 16),
            1 => (44100, 16),
            2 => (44100, 16),
            _ => (44100, 16),
        }
    }

    fn parse_track(&self, item: &Value) -> Option<UnifiedTrack> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        let duration = item.get("duration")?.as_u64()? as u32;
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        let album = item.get("album").and_then(|a| self.parse_album(a));
        
        // Determine available qualities
        let qualities_available = vec![Quality::Normal, Quality::LosslessFlac];

        Some(UnifiedTrack {
            id,
            title,
            duration,
            track_number: item.get("track_position").and_then(|n| n.as_u64()).map(|n| n as u32),
            volume_number: item.get("disk_number").and_then(|n| n.as_u64()).map(|n| n as u32),
            replay_gain: None,
            peak: None,
            available: true,
            audio_quality: Some("LOSSLESS".to_string()),
            audio_modes: None,
            artist,
            artists: item.get("contributors").and_then(|a| a.as_array()).map(|arr| {
                arr.iter().filter_map(|item| self.parse_artist(item)).collect()
            }),
            album,
            source: MusicSource::Deezer,
            qualities_available,
        })
    }

    fn parse_album(&self, item: &Value) -> Option<UnifiedAlbum> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        
        Some(UnifiedAlbum {
            id,
            title,
            cover: item.get("cover").and_then(|i| i.as_str()).map(|s| s.to_string())
                .or_else(|| item.get("cover_medium").and_then(|i| i.as_str()).map(|s| s.to_string()))
                .or_else(|| item.get("cover_big").and_then(|i| i.as_str()).map(|s| s.to_string())),
            duration: item.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32),
            track_count: item.get("nb_tracks").and_then(|c| c.as_u64()).map(|c| c as u32),
            release_date: item.get("release_date").and_then(|d| d.as_str()).map(|s| s.to_string()),
            artist,
            artists: None,
            url: item.get("link").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::Deezer,
        })
    }

    fn parse_artist(&self, item: &Value) -> Option<UnifiedArtist> {
        Some(UnifiedArtist {
            id: item.get("id")?.as_u64()?.to_string(),
            name: item.get("name")?.as_str()?.to_string(),
            picture: item.get("picture").and_then(|i| i.as_str()).map(|s| s.to_string())
                .or_else(|| item.get("picture_medium").and_then(|i| i.as_str()).map(|s| s.to_string()))
                .or_else(|| item.get("picture_big").and_then(|i| i.as_str()).map(|s| s.to_string())),
            url: item.get("link").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::Deezer,
        })
    }
}
