use crate::types::*;
use reqwest::Client;
use serde_json::Value;
use lazy_static::lazy_static;
use rand::Rng;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

const SOUNDCLOUD_BASE_URL: &str = "https://api-v2.soundcloud.com";
const SOUNDCLOUD_WEB_URL: &str = "https://soundcloud.com";

const NON_STREAMABLE: &str = "_non_streamable";
const ORIGINAL_DOWNLOAD: &str = "_original_download";
const NOT_RESOLVED: &str = "_not_resolved";

/// SoundCloud client for MP3 streaming
pub struct SoundCloudClient {
    client_id: String,
    app_version: String,
    user_id: String,
}

impl SoundCloudClient {
    pub fn new(client_id: String, app_version: String) -> Self {
        let mut rng = rand::thread_rng();
        let user_id = format!("{}-{}-{}-{}",
            rng.gen_range(111111..999999),
            rng.gen_range(111111..999999),
            rng.gen_range(111111..999999),
            rng.gen_range(111111..999999)
        );

        Self {
            client_id,
            app_version,
            user_id,
        }
    }

    /// Auto-fetch client_id and app_version from SoundCloud
    pub async fn new_auto() -> Result<Self, String> {
        let (client_id, app_version) = Self::fetch_credentials().await?;
        Ok(Self::new(client_id, app_version))
    }

    /// Fetch credentials from SoundCloud web page
    async fn fetch_credentials() -> Result<(String, String), String> {
        let response = HTTP_CLIENT
            .get(SOUNDCLOUD_WEB_URL)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch SoundCloud page: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Extract script URL
        let script_url = html
            .split("<script crossorigin src=\"")
            .nth(1)
            .and_then(|s| s.split("\"").next())
            .ok_or("Could not find script URL")?;

        let script_url = if script_url.starts_with("http") {
            script_url.to_string()
        } else {
            format!("https://{}", script_url)
        };

        // Fetch script
        let script_response = HTTP_CLIENT
            .get(&script_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch script: {}", e))?;

        let script = script_response.text().await
            .map_err(|e| format!("Failed to read script: {}", e))?;

        // Extract app version
        let app_version = script
            .split("window.__sc_version=\"")
            .nth(1)
            .and_then(|s| s.split("\"").next())
            .ok_or("Could not find app version")?
            .to_string();

        // Extract client ID
        let client_id = script
            .split("client_id:")
            .nth(1)
            .and_then(|s| s.split("\"").nth(1))
            .ok_or("Could not find client ID")?
            .to_string();

        Ok((client_id, app_version))
    }

    /// Refresh credentials
    pub async fn refresh_credentials(&mut self) -> Result<(), String> {
        let (client_id, app_version) = Self::fetch_credentials().await?;
        self.client_id = client_id;
        self.app_version = app_version;
        Ok(())
    }

    /// Verify credentials are valid
    pub async fn verify_credentials(&self) -> Result<bool, String> {
        let url = format!("{}/announcements", SOUNDCLOUD_BASE_URL);
        
        let response = HTTP_CLIENT
            .get(&url)
            .query(&[
                ("client_id", &self.client_id),
                ("app_version", &self.app_version),
            ])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        Ok(response.status().is_success())
    }

    /// Search for tracks on SoundCloud
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let url = format!("{}/search/tracks", SOUNDCLOUD_BASE_URL);
        
        let params = [
            ("q", query),
            ("limit", &limit.to_string()),
            ("offset", "0"),
            ("linked_partitioning", "1"),
            ("facet", "genre"),
            ("user_id", &self.user_id),
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

        if let Some(collection) = data.get("collection").and_then(|c| c.as_array()) {
            let parsed: Vec<UnifiedTrack> = collection.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
            return Ok(parsed);
        }

        Ok(vec![])
    }

    /// Search for playlists on SoundCloud
    pub async fn search_playlists(&self, query: &str, limit: u32) -> Result<Vec<UnifiedPlaylist>, String> {
        let url = format!("{}/search/playlists", SOUNDCLOUD_BASE_URL);
        
        let params = [
            ("q", query),
            ("limit", &limit.to_string()),
            ("offset", "0"),
            ("linked_partitioning", "1"),
            ("facet", "genre"),
            ("user_id", &self.user_id),
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

        if let Some(collection) = data.get("collection").and_then(|c| c.as_array()) {
            let parsed: Vec<UnifiedPlaylist> = collection.iter()
                .filter_map(|item| self.parse_playlist(item))
                .collect();
            return Ok(parsed);
        }

        Ok(vec![])
    }

    /// Get trending tracks from SoundCloud
    pub async fn get_trending_tracks(&self, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let url = format!("{}/tracks", SOUNDCLOUD_BASE_URL);
        
        let params = [
            ("limit", &limit.to_string()),
            ("offset", &String::from("0")),
            ("linked_partitioning", &String::from("1")),
            ("user_id", &self.user_id),
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

        if let Some(collection) = data.get("collection").and_then(|c| c.as_array()) {
            let parsed: Vec<UnifiedTrack> = collection.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
            return Ok(parsed);
        }

        Ok(vec![])
    }

    /// Get track details
    pub async fn get_track(&self, track_id: &str) -> Result<UnifiedTrack, String> {
        let url = format!("{}/tracks/{}", SOUNDCLOUD_BASE_URL, track_id);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("client_id", &self.client_id)])
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

    /// Get playlist details
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<UnifiedPlaylist, String> {
        let url = format!("{}/playlists/{}", SOUNDCLOUD_BASE_URL, playlist_id);

        let response = HTTP_CLIENT
            .get(&url)
            .query(&[("client_id", &self.client_id)])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        self.parse_playlist(&data).ok_or("Failed to parse playlist".to_string())
    }

    /// Get stream URL for a track
    pub async fn get_stream_url(&self, track_id: &str, _quality: u8) -> Result<StreamInfo, String> {
        // Parse custom ID format
        let parts: Vec<&str> = track_id.split('|').collect();
        if parts.len() != 2 {
            return Err("Invalid track ID format".to_string());
        }

        let actual_id = parts[0];
        let download_info = parts[1];

        match download_info {
            NON_STREAMABLE => Err("Track is not streamable".to_string()),
            ORIGINAL_DOWNLOAD => {
                let url = format!("{}/tracks/{}/download", SOUNDCLOUD_BASE_URL, actual_id);
                
                let response = HTTP_CLIENT
                    .get(&url)
                    .query(&[("client_id", &self.client_id)])
                    .send()
                    .await
                    .map_err(|e| format!("Request failed: {}", e))?;

                if !response.status().is_success() {
                    return Err(format!("HTTP error: {}", response.status()));
                }

                let data: Value = response.json().await
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                let stream_url = data.get("redirectUri")
                    .and_then(|u| u.as_str())
                    .ok_or("No redirect URI found")?;

                Ok(StreamInfo {
                    track_id: track_id.to_string(),
                    quality: Quality::Normal, // Assume 320kbps for original
                    codec: "mp3".to_string(),
                    url: stream_url.to_string(),
                    encryption_key: None,
                    source: MusicSource::SoundCloud,
                    bitrate: 320,
                    sample_rate: 44100,
                    bit_depth: 16,
                })
            }
            NOT_RESOLVED => Err("Track not resolved".to_string()),
            stream_url => {
                // This is an HLS stream URL
                let response = HTTP_CLIENT
                    .get(stream_url)
                    .send()
                    .await
                    .map_err(|e| format!("Request failed: {}", e))?;

                if !response.status().is_success() {
                    return Err(format!("HTTP error: {}", response.status()));
                }

                let data: Value = response.json().await
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                let final_url = data.get("url")
                    .and_then(|u| u.as_str())
                    .ok_or("No stream URL found")?;

                Ok(StreamInfo {
                    track_id: track_id.to_string(),
                    quality: Quality::Normal, // Assume 320kbps
                    codec: "mp3".to_string(),
                    url: final_url.to_string(),
                    encryption_key: None,
                    source: MusicSource::SoundCloud,
                    bitrate: 320,
                    sample_rate: 44100,
                    bit_depth: 16,
                })
            }
        }
    }

    /// Resolve a URL to metadata
    pub async fn resolve_url(&self, url: &str) -> Result<Value, String> {
        let api_url = format!("{}/resolve", SOUNDCLOUD_BASE_URL);
        
        let response = HTTP_CLIENT
            .get(&api_url)
            .query(&[
                ("url", url),
                ("client_id", &self.client_id),
            ])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    fn parse_track(&self, item: &Value) -> Option<UnifiedTrack> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        let duration = item.get("duration")?.as_u64()? as u32;

        let artist = item.get("user").and_then(|u| self.parse_artist(u));
        
        // Generate custom ID with download info
        let custom_id = self.get_custom_id(item);

        let qualities_available = vec![Quality::Normal]; // SoundCloud only has MP3

        Some(UnifiedTrack {
            id: custom_id,
            title,
            duration,
            track_number: None,
            volume_number: None,
            replay_gain: None,
            peak: None,
            available: item.get("streamable").and_then(|s| s.as_bool()).unwrap_or(false),
            audio_quality: Some("NORMAL".to_string()),
            audio_modes: None,
            artist: artist.clone(),
            artists: artist.map(|a| vec![a]),
            album: None,
            source: MusicSource::SoundCloud,
            qualities_available,
        })
    }

    fn parse_playlist(&self, item: &Value) -> Option<UnifiedPlaylist> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        
        let creator = item.get("user").and_then(|u| {
            Some(Creator {
                id: u.get("id")?.as_u64()?.to_string(),
                name: u.get("username")?.as_str()?.to_string(),
            })
        });

        let track_count = item.get("track_count").and_then(|t| t.as_u64()).map(|t| t as u32);
        let duration = item.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32);

        Some(UnifiedPlaylist {
            id,
            title,
            description: item.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            picture: item.get("artwork_url").and_then(|a| a.as_str()).map(|s| s.to_string()),
            duration,
            track_count,
            last_updated: item.get("last_modified").and_then(|d| d.as_str()).map(|s| s.to_string()),
            creator,
            source: MusicSource::SoundCloud,
        })
    }

    fn parse_artist(&self, item: &Value) -> Option<UnifiedArtist> {
        Some(UnifiedArtist {
            id: item.get("id")?.as_u64()?.to_string(),
            name: item.get("username")?.as_str()?.to_string(),
            picture: item.get("avatar_url").and_then(|a| a.as_str()).map(|s| s.to_string()),
            url: item.get("permalink_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::SoundCloud,
        })
    }

    fn get_custom_id(&self, item: &Value) -> String {
        let id = item.get("id")
            .and_then(|i| i.as_u64())
            .map(|i| i.to_string())
            .unwrap_or_default();

        let streamable = item.get("streamable")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        let policy = item.get("policy")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if !streamable || policy == "BLOCK" {
            return format!("{}|{}", id, NON_STREAMABLE);
        }

        let downloadable = item.get("downloadable")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        let has_downloads = item.get("has_downloads_left")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);

        if downloadable && has_downloads {
            return format!("{}|{}", id, ORIGINAL_DOWNLOAD);
        }

        // Find HLS transcoding
        if let Some(media) = item.get("media").and_then(|m| m.as_array()) {
            for transcoding in media {
                if let Some(transcodings) = transcoding.get("transcodings").and_then(|t| t.as_array()) {
                    for tc in transcodings {
                        if let Some(format) = tc.get("format") {
                            let protocol = format.get("protocol").and_then(|p| p.as_str()).unwrap_or("");
                            let mime_type = format.get("mime_type").and_then(|m| m.as_str()).unwrap_or("");
                            
                            if protocol == "hls" && mime_type == "audio/mpeg" {
                                if let Some(url) = tc.get("url").and_then(|u| u.as_str()) {
                                    return format!("{}|{}", id, url);
                                }
                            }
                        }
                    }
                }
            }
        }

        format!("{}|{}", id, NOT_RESOLVED)
    }
}
