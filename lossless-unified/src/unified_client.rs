use crate::types::*;
use reqwest::Client;
use serde_json::Value;
use lazy_static::lazy_static;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
}

const UNIFIED_API_BASE_URL: &str = "https://music-api.geeked.wtf";
const UNIFIED_API_TOKEN: &str = "amp_29b2lIr4mze4tK-P8QDOxfMZ9anCgJ9_uGTUks3nIyo";
const TURNSTILE_SITE_KEY: &str = "0x4AAAAAADgxqF6QVMm0GLHH";

/// Unified Playback API client
/// This provides high-quality streaming from multiple sources via a single API
pub struct UnifiedClient {
    api_base_url: String,
    api_token: String,
    turnstile_jwt: Option<String>,
}

impl UnifiedClient {
    pub fn new(api_base_url: Option<String>, api_token: Option<String>) -> Self {
        Self {
            api_base_url: api_base_url.unwrap_or_else(|| UNIFIED_API_BASE_URL.to_string()),
            api_token: api_token.unwrap_or_else(|| UNIFIED_API_TOKEN.to_string()),
            turnstile_jwt: None,
        }
    }

    /// Set Turnstile JWT for rate limit protection
    pub fn set_turnstile_jwt(&mut self, jwt: String) {
        self.turnstile_jwt = Some(jwt);
    }

    /// Get stream URL via Unified Playback API
    pub async fn get_stream_url(&self, isrc: &str, quality: Quality) -> Result<StreamInfo, String> {
        let quality_str = self.quality_to_unified_format(quality);
        
        let url = format!("{}/api/v2/track/", self.api_base_url);
        
        let mut request = HTTP_CLIENT
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .query(&[
                ("isrc", isrc),
                ("quality", &quality_str),
                ("intent", "stream"),
            ]);

        // Add Turnstile JWT if available
        if let Some(jwt) = &self.turnstile_jwt {
            request = request.header("X-Turnstile-JWT", jwt);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Parse response based on version
        let version = data.get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        if version == 2 {
            self.parse_v2_response(&data, isrc).await
        } else {
            self.parse_v1_response(&data, isrc).await
        }
    }

    async fn parse_v1_response(&self, data: &Value, isrc: &str) -> Result<StreamInfo, String> {
        let envelope = data.get("envelope")
            .ok_or("No envelope in response")?;

        let playback = envelope.get("playback")
            .and_then(|p| p.as_array())
            .ok_or("No playback data")?;

        // Find the best quality playback resource
        let best_resource = playback.iter()
            .filter(|item| {
                let kind = item.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                kind == "audio" || kind == "manifest"
            })
            .filter_map(|item| {
                let quality_str = item.get("quality").and_then(|q| q.as_str()).unwrap_or("");
                let quality = Quality::from_str(quality_str);
                Some((quality, item))
            })
            .max_by_key(|(quality, _)| *quality);

        let (_, resource) = best_resource.ok_or("No suitable playback resource found")?;

        let url = resource.get("url")
            .and_then(|u| u.as_str())
            .ok_or("No URL in playback resource")?;

        let codec = resource.get("codec")
            .and_then(|c| c.as_str())
            .unwrap_or("flac");

        let quality_str = resource.get("quality")
            .and_then(|q| q.as_str())
            .unwrap_or("LOSSLESS");

        let encryption_key = resource.get("key_id")
            .and_then(|k| k.as_str())
            .map(|s| s.to_string());

        let sample_rate = resource.get("sample_rate")
            .and_then(|s| s.as_u64())
            .unwrap_or(44100) as u32;

        let bit_depth = resource.get("bit_depth")
            .and_then(|b| b.as_u64())
            .unwrap_or(16) as u8;

        Ok(StreamInfo {
            track_id: isrc.to_string(),
            quality: Quality::from_str(quality_str),
            codec: codec.to_string(),
            url: url.to_string(),
            encryption_key,
            source: MusicSource::UnifiedPlayback,
            bitrate: Quality::from_str(quality_str).bitrate(),
            sample_rate,
            bit_depth,
        })
    }

    async fn parse_v2_response(&self, data: &Value, isrc: &str) -> Result<StreamInfo, String> {
        let playback = data.get("playback")
            .and_then(|p| p.as_array())
            .ok_or("No playback data")?;

        // Find the best quality playback resource
        let best_resource = playback.iter()
            .filter(|item| {
                let kind = item.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                kind == "audio" || kind == "manifest"
            })
            .filter_map(|item| {
                let quality_str = item.get("quality").and_then(|q| q.as_str()).unwrap_or("");
                let quality = Quality::from_str(quality_str);
                Some((quality, item))
            })
            .max_by_key(|(quality, _)| *quality);

        let (_, resource) = best_resource.ok_or("No suitable playback resource found")?;

        let url = resource.get("url")
            .and_then(|u| u.as_str())
            .ok_or("No URL in playback resource")?;

        let codec = resource.get("codec")
            .and_then(|c| c.as_str())
            .unwrap_or("flac");

        let quality_str = resource.get("quality")
            .and_then(|q| q.as_str())
            .unwrap_or("LOSSLESS");

        let encryption_key = resource.get("key_id")
            .and_then(|k| k.as_str())
            .map(|s| s.to_string());

        let sample_rate = resource.get("sample_rate")
            .and_then(|s| s.as_u64())
            .unwrap_or(44100) as u32;

        let bit_depth = resource.get("bit_depth")
            .and_then(|b| b.as_u64())
            .unwrap_or(16) as u8;

        Ok(StreamInfo {
            track_id: isrc.to_string(),
            quality: Quality::from_str(quality_str),
            codec: codec.to_string(),
            url: url.to_string(),
            encryption_key,
            source: MusicSource::UnifiedPlayback,
            bitrate: Quality::from_str(quality_str).bitrate(),
            sample_rate,
            bit_depth,
        })
    }

    /// Search via Unified API
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<UnifiedTrack>, String> {
        let url = format!("{}/api/v2/search", self.api_base_url);
        
        let response = HTTP_CLIENT
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .query(&[
                ("query", query),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data: Value = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Parse tracks from response
        if let Some(tracks) = data.get("tracks").and_then(|t| t.as_array()) {
            return tracks.iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
        }

        Ok(vec![])
    }

    fn parse_track(&self, item: &Value) -> Option<UnifiedTrack> {
        let id = item.get("id")?.as_str()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        let duration = item.get("duration")?.as_u64()? as u32;

        let artist = item.get("artist").and_then(|a| self.parse_artist(a));
        let album = item.get("album").and_then(|a| self.parse_album(a));

        let audio_quality = item.get("audio_quality")
            .and_then(|q| q.as_str())
            .map(|s| s.to_string());

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
            audio_quality,
            audio_modes: None,
            artist,
            artists: None,
            album,
            source: MusicSource::UnifiedPlayback,
            qualities_available,
        })
    }

    fn parse_artist(&self, item: &Value) -> Option<UnifiedArtist> {
        Some(UnifiedArtist {
            id: item.get("id")?.as_str()?.to_string(),
            name: item.get("name")?.as_str()?.to_string(),
            picture: item.get("picture").and_then(|p| p.as_str()).map(|s| s.to_string()),
            url: item.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::UnifiedPlayback,
        })
    }

    fn parse_album(&self, item: &Value) -> Option<UnifiedAlbum> {
        Some(UnifiedAlbum {
            id: item.get("id")?.as_str()?.to_string(),
            title: item.get("title")?.as_str()?.to_string(),
            cover: item.get("cover").and_then(|c| c.as_str()).map(|s| s.to_string()),
            duration: item.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32),
            track_count: item.get("track_count").and_then(|t| t.as_u64()).map(|t| t as u32),
            release_date: item.get("release_date").and_then(|d| d.as_str()).map(|s| s.to_string()),
            artist: item.get("artist").and_then(|a| self.parse_artist(a)),
            artists: None,
            url: item.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
            source: MusicSource::UnifiedPlayback,
        })
    }

    fn quality_to_unified_format(&self, quality: Quality) -> &'static str {
        match quality {
            Quality::Low => "LOW",
            Quality::Normal => "HIGH",
            Quality::LosslessFlac => "LOSSLESS",
            Quality::HiRes => "HI_RES_LOSSLESS",
            Quality::UltraHiRes => "HI_RES_LOSSLESS",
            Quality::DolbyAtmos => "DOLBY_ATMOS",
        }
    }
}
