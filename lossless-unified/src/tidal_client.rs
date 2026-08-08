use crate::types::*;
use crate::proxy::{get_tidal_client_credentials, get_monochrome_tidal_credentials, get_streamrip_tidal_credentials};
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose};
use serde_json::Value;
use lazy_static::lazy_static;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

const TIDAL_BASE_URL: &str = "https://api.tidalhifi.com/v1";
const TIDAL_OPENAPI_URL: &str = "https://openapi.tidal.com/v2";
const AUTH_URL: &str = "https://auth.tidal.com/v1/oauth2";

const QUALITY_MAP: &[&str] = &["LOW", "HIGH", "LOSSLESS", "HI_RES", "HI_RES_LOSSLESS", "DOLBY_ATMOS"];

/// Tidal client for MQA/FLAC streaming
#[derive(Clone)]
pub struct TidalClient {
    access_token: String,
    country_code: String,
    user_id: String,
    token_expiry: Option<f64>,
    refresh_token: Option<String>,
}

impl TidalClient {
    /// Create a new Tidal client with automatic OAuth token fetch
    /// Tries multiple credential sources with fallback
    pub async fn new_auto() -> Result<Self, String> {
        // Try multiple credential sources with fallback
        let credential_sources = vec![
            get_monochrome_tidal_credentials(),
            get_streamrip_tidal_credentials(),
            get_tidal_client_credentials(),
        ];
        
        let mut last_error = String::new();
        
        for (client_id, client_secret) in credential_sources {
            match Self::try_oauth(&client_id, &client_secret).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    last_error = format!("Failed with client_id {}: {}", &client_id[..8], e);
                    continue;
                }
            }
        }
        
        Err(format!("All credential sources failed. Last error: {}", last_error))
    }
    
    /// Try OAuth with specific credentials
    async fn try_oauth(client_id: &str, client_secret: &str) -> Result<Self, String> {
        // Fetch OAuth token using client credentials (streamrip approach)
        let auth_header = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", client_id, client_secret)));
        
        let params = [
            ("grant_type", "client_credentials"),
        ];
        
        let response = HTTP_CLIENT
            .post(&format!("{}/token", AUTH_URL))
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("OAuth request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("OAuth HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("OAuth JSON parse error: {}", e))?;

        let access_token = data.get("access_token")
            .and_then(|t| t.as_str())
            .ok_or("No access token in OAuth response")?
            .to_string();

        let expires_in = data.get("expires_in")
            .and_then(|e| e.as_u64())
            .unwrap_or(3600);

        let token_expiry = Some((std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64() + expires_in as f64));

        Ok(Self {
            access_token,
            country_code: "US".to_string(),
            user_id: "default".to_string(),
            token_expiry,
            refresh_token: None,
        })
    }

    pub fn new(access_token: String, country_code: String, user_id: String) -> Self {
        Self {
            access_token,
            country_code,
            user_id,
            token_expiry: None,
            refresh_token: None,
        }
    }

    /// Check if token needs refresh
    pub fn needs_refresh(&self) -> bool {
        if let Some(expiry) = self.token_expiry {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            now >= expiry - 300.0 // Refresh 5 minutes before expiry
        } else {
            false
        }
    }

    /// Refresh the access token
    pub async fn refresh_token(&mut self) -> Result<(), String> {
        if let Some(refresh_token) = &self.refresh_token {
            let (client_id, client_secret) = get_tidal_client_credentials();
            
            let params = [
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ];
            
            let response = HTTP_CLIENT
                .post(&format!("{}/token", AUTH_URL))
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Token refresh failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Token refresh HTTP error: {}", response.status()));
            }

            let data: Value = response.json().await
                .map_err(|e| format!("Token refresh JSON parse error: {}", e))?;

            self.access_token = data.get("access_token")
                .and_then(|t| t.as_str())
                .ok_or("No access token in refresh response")?
                .to_string();

            if let Some(new_refresh) = data.get("refresh_token").and_then(|r| r.as_str()) {
                self.refresh_token = Some(new_refresh.to_string());
            }

            let expires_in = data.get("expires_in")
                .and_then(|e| e.as_u64())
                .unwrap_or(3600);

            self.token_expiry = Some((std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() + expires_in as f64));

            Ok(())
        } else {
            // Fall back to client credentials flow
            let new_client = Self::new_auto().await?;
            self.access_token = new_client.access_token;
            self.token_expiry = new_client.token_expiry;
            Ok(())
        }
    }

    /// Search for tracks on Tidal
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
            "countryCode": self.country_code,
        });

        let response = self.api_request("search/tracks", &params).await?;
        
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
        }
        
        Ok(vec![])
    }

    /// Search for albums on Tidal
    pub async fn search_albums(&self, query: &str, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
            "countryCode": self.country_code,
        });

        let response = self.api_request("search/albums", &params).await?;
        
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_album(item))
                .collect();
        }
        
        Ok(vec![])
    }

    /// Search for artists on Tidal
    pub async fn search_artists(&self, query: &str, limit: u32) -> Result<Vec<UnifiedArtist>, String> {
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
            "countryCode": self.country_code,
        });

        let response = self.api_request("search/artists", &params).await?;
        
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_artist(item))
                .collect();
        }
        
        Ok(vec![])
    }

    /// Get track details
    pub async fn get_track(&self, track_id: &str) -> Result<UnifiedTrack, String> {
        let params = serde_json::json!({
            "countryCode": self.country_code,
        });

        let response = self.api_request(&format!("tracks/{}", track_id), &params).await?;
        self.parse_track(&response).ok_or("Failed to parse track".to_string())
    }

    /// Get album details
    pub async fn get_album(&self, album_id: &str) -> Result<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "countryCode": self.country_code,
        });

        let response = self.api_request(&format!("albums/{}", album_id), &params).await?;
        self.parse_album(&response).ok_or("Failed to parse album".to_string())
    }

    /// Get featured playlists from Tidal
    pub async fn get_featured_playlists(&self, limit: u32) -> Result<Vec<UnifiedPlaylist>, String> {
        let params = serde_json::json!({
            "limit": limit,
            "countryCode": self.country_code,
        });

        let response = self.api_request("playlists", &params).await?;
        
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_playlist(item))
                .collect();
        }
        
        Ok(vec![])
    }
    
    /// Get featured/new releases from Tidal
    pub async fn get_featured(&self, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "limit": limit,
            "countryCode": self.country_code,
            "type": "new",
        });

        let response = self.api_request("pages/new", &params).await?;
        
        // Parse featured albums from response
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_album(item))
                .collect();
        }
        
        Ok(vec![])
    }

    /// Get top tracks from Tidal
    pub async fn get_top_tracks(&self, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let params = serde_json::json!({
            "limit": limit,
            "countryCode": self.country_code,
        });

        let response = self.api_request("tracks/top", &params).await?;
        
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
        }
        
        Ok(vec![])
    }

    /// Get stream URL for a track at specified quality
    pub async fn get_stream_url(&self, track_id: &str, quality: u8) -> Result<StreamInfo, String> {
        // Auto-refresh token if needed
        if self.needs_refresh() {
            let mut mutable_self = self.clone();
            mutable_self.refresh_token().await?;
        }

        let quality_str = QUALITY_MAP.get(quality as usize).unwrap_or(&"LOSSLESS");
        
        let params = serde_json::json!({
            "audioquality": quality_str,
            "playbackmode": "STREAM",
            "assetpresentation": "FULL",
            "countryCode": self.country_code,
        });

        let response = self.api_request(
            &format!("tracks/{}/playbackinfopostpaywall", track_id),
            &params
        ).await?;

        let manifest_base64 = response.get("manifest")
            .and_then(|m| m.as_str())
            .ok_or("No manifest found")?;

        let manifest_bytes = general_purpose::STANDARD
            .decode(manifest_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;

        let manifest_str = String::from_utf8(manifest_bytes)
            .map_err(|e| format!("UTF-8 decode error: {}", e))?;

        let manifest: Value = serde_json::from_str(&manifest_str)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let url = manifest.get("urls")
            .and_then(|u| u.as_array())
            .and_then(|arr| arr.first())
            .and_then(|u| u.as_str())
            .ok_or("No stream URL found in manifest")?;

        let codec = manifest.get("codecs")
            .and_then(|c| c.as_str())
            .unwrap_or("flac");

        let encryption_key = manifest.get("keyId").and_then(|k| k.as_str()).map(|s| s.to_string());

        // Detect Dolby Atmos
        let is_dolby_atmos = codec == "eac3_joc" || codec == "ec-3";
        let actual_quality = if is_dolby_atmos {
            Quality::DolbyAtmos
        } else {
            Quality::from_number(quality.min(4))
        };

        let (sample_rate, bit_depth) = self.quality_to_sample_params(quality);

        Ok(StreamInfo {
            track_id: track_id.to_string(),
            quality: actual_quality,
            codec: codec.to_string(),
            url: url.to_string(),
            encryption_key,
            source: MusicSource::Tidal,
            bitrate: actual_quality.bitrate(),
            sample_rate,
            bit_depth,
        })
    }

    /// Get featured/new releases
    pub async fn get_featured(&self, limit: u32) -> Result<Vec<UnifiedAlbum>, String> {
        let params = serde_json::json!({
            "limit": limit,
            "countryCode": self.country_code,
            "type": "new",
        });

        let response = self.api_request("pages/new", &params).await?;
        
        // Parse featured albums from response
        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            return items.iter()
                .filter_map(|item| self.parse_album(item))
                .collect();
        }
        
        Ok(vec![])
    }

    async fn api_request(&self, endpoint: &str, params: &Value) -> Result<Value, String> {
        let url = format!("{}/{}", TIDAL_BASE_URL, endpoint);

        let response = HTTP_CLIENT
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .query(params)
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
        let duration = item.get("duration")?.as_f64()? as u32 * 1000; // Convert to ms
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        let album = item.get("album").and_then(|a| self.parse_album(a));
        
        // Determine available qualities based on audio quality
        let audio_quality = item.get("audioQuality").and_then(|q| q.as_str()).unwrap_or("LOSSLESS");
        let qualities_available = match audio_quality {
            "HI_RES" => vec![Quality::High, Quality::HiRes],
            "LOSSLESS" => vec![Quality::High],
            _ => vec![Quality::Normal, Quality::High],
        };

        Some(UnifiedTrack {
            id,
            title,
            duration,
            track_number: item.get("trackNumber").and_then(|n| n.as_u64()).map(|n| n as u32),
            volume_number: item.get("volumeNum").and_then(|n| n.as_u64()).map(|n| n as u32),
            replay_gain: None,
            peak: None,
            available: true,
            audio_quality: Some(audio_quality.to_string()),
            audio_modes: item.get("audioModes").and_then(|m| m.as_array()).map(|arr| {
                arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()
            }),
            artist,
            artists: item.get("artists").and_then(|a| a.as_array()).map(|arr| {
                arr.iter().filter_map(|item| self.parse_artist(item)).collect()
            }),
            album,
            source: MusicSource::Tidal,
            qualities_available,
        })
    }

    fn parse_playlist(&self, item: &Value) -> Option<UnifiedPlaylist> {
        let id = item.get("uuid")?.as_str()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        
        let creator = item.get("creator").and_then(|c| {
            Some(Creator {
                id: c.get("id")?.as_u64()?.to_string(),
                name: c.get("name")?.as_str()?.to_string(),
            })
        });
        
        Some(UnifiedPlaylist {
            id,
            title,
            description: item.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            picture: item.get("image").and_then(|i| i.as_str()).map(|s| s.to_string()),
            duration: item.get("duration").and_then(|d| d.as_f64()).map(|d| d as u32 * 1000),
            track_count: item.get("numberOfTracks").and_then(|c| c.as_u64()).map(|c| c as u32),
            last_updated: item.get("lastUpdated").and_then(|d| d.as_str()).map(|s| s.to_string()),
            creator,
            source: MusicSource::Tidal,
        })
    }

    fn parse_album(&self, item: &Value) -> Option<UnifiedAlbum> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        
        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        
        Some(UnifiedAlbum {
            id,
            title,
            cover: item.get("cover").and_then(|i| i.as_str()).map(|s| s.to_string()),
            duration: item.get("duration").and_then(|d| d.as_f64()).map(|d| d as u32 * 1000),
            track_count: item.get("numberOfTracks").and_then(|c| c.as_u64()).map(|c| c as u32),
            release_date: item.get("releaseDate").and_then(|d| d.as_str()).map(|s| s.to_string()),
            artist,
            artists: item.get("artists").and_then(|a| a.as_array()).map(|arr| {
                arr.iter().filter_map(|item| self.parse_artist(item)).collect()
            }),
            url: item.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::Tidal,
        })
    }

    fn parse_artist(&self, item: &Value) -> Option<UnifiedArtist> {
        Some(UnifiedArtist {
            id: item.get("id")?.as_u64()?.to_string(),
            name: item.get("name")?.as_str()?.to_string(),
            picture: item.get("picture").and_then(|i| i.as_str()).map(|s| s.to_string()),
            url: None,
            source: MusicSource::Tidal,
        })
    }

    fn quality_to_sample_params(&self, quality: u8) -> (u32, u8) {
        match quality {
            0 => (44100, 16),
            1 => (44100, 16),
            2 => (44100, 16),
            3 => (96000, 24),
            _ => (44100, 16),
        }
    }
}
