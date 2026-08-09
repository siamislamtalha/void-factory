use crate::types::*;
use crate::client::{ensure_clients_initialized, QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
use crate::proxy::{get_unified_api_credentials, get_tidal_client_credentials};
use reqwest::Client;
use serde_json::Value;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
    static ref API_CACHE: Mutex<HashMap<String, (Value, Instant)>> = Mutex::new(HashMap::new());
    static ref UNIFIED_API_TOKEN: String = {
        let (token, _, _) = get_unified_api_credentials();
        token
    };
    static ref UNIFIED_API_BASE_URL: String = {
        let (_, url, _) = get_unified_api_credentials();
        url
    };
}

/// Monochrome-compatible API server
/// This module implements Monochrome-style API endpoints while preserving
/// advanced features like parallel search and quality hierarchy
/// Uses actual Monochrome API credentials and caching mechanisms
/// Implements the full Monochrome API interface for seamless integration

const CACHE_TTL: Duration = Duration::from_secs(1800); // 30 minutes cache

/// Get cached value if available and not expired
fn get_cached(key: &str) -> Option<Value> {
    let cache = API_CACHE.lock().unwrap();
    if let Some((value, timestamp)) = cache.get(key) {
        if timestamp.elapsed() < CACHE_TTL {
            return Some(value.clone());
        }
    }
    None
}

/// Set cached value
fn set_cached(key: String, value: Value) {
    let mut cache = API_CACHE.lock().unwrap();
    cache.insert(key, (value, Instant::now()));
}

/// Fetch from Monochrome API instances with retry logic (Monochrome-style)
async fn fetch_from_monochrome(path: &str) -> Result<Value, String> {
    let instances = crate::proxy::get_instances("api");
    if instances.is_empty() {
        return Err("No Monochrome API instances configured".to_string());
    }
    
    let mut last_error = String::new();
    
    for instance in instances {
        let base_url = instance.url.trim_end_matches('/');
        let url = format!("{}{}", base_url, path);
        
        match HTTP_CLIENT.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: Value = response.json().await
                        .map_err(|e| format!("JSON parse error: {}", e))?;
                    return Ok(data);
                } else if response.status() == 429 {
                    last_error = format!("Rate limit hit on {}", base_url);
                    continue;
                } else if response.status() >= 500 {
                    last_error = format!("Server error {} on {}", response.status(), base_url);
                    continue;
                } else {
                    last_error = format!("HTTP {} from {}", response.status(), base_url);
                    continue;
                }
            }
            Err(e) => {
                last_error = format!("Request failed to {}: {}", base_url, e);
                continue;
            }
        }
    }
    
    Err(format!("All Monochrome API instances failed. Last error: {}", last_error))
}

/// Fetch from Unified Playback API (Monochrome's unified backend)
async fn fetch_from_unified(path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let url = format!("{}{}", UNIFIED_API_BASE_URL.as_str(), path);
    
    let mut request = HTTP_CLIENT
        .get(&url)
        .header("Authorization", format!("Bearer {}", UNIFIED_API_TOKEN.as_str()));
    
    if !params.is_empty() {
        request = request.query(params);
    }
    
    let response = request.send().await
        .map_err(|e| format!("Unified API request failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Unified API HTTP error: {}", response.status()));
    }
    
    response.json().await
        .map_err(|e| format!("Unified API JSON parse error: {}", e))
}

/// Get track info in Monochrome format
/// Endpoint: /info/?id={id}
/// Uses Monochrome API instances with caching fallback
pub async fn get_track_info(id: &str) -> Result<MonochromeResponse<MonochromeTrack>, String> {
    ensure_clients_initialized().await;
    
    // Check cache first
    let cache_key = format!("track_{}", id);
    if let Some(cached) = get_cached(&cache_key) {
        if let Ok(track) = serde_json::from_value::<MonochromeTrack>(cached.clone()) {
            return Ok(MonochromeResponse {
                data: Some(track),
                error: None,
            });
        }
    }
    
    // Parse source from ID if prefixed
    let (source, actual_id) = if id.contains(':') {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
    } else {
        // Default to Tidal for Monochrome compatibility
        (Some(MusicSource::Tidal), Some(id.to_string()))
    };
    
    let (source, actual_id) = match (source, actual_id) {
        (Some(s), Some(id)) => (s, id),
        _ => return Err("Invalid track ID format".to_string()),
    };
    
    let track = match source {
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_track(&actual_id).await?;
                convert_to_monochrome_track(unified)
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_track(&actual_id).await?;
                convert_to_monochrome_track(unified)
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_track(&actual_id).await?;
                convert_to_monochrome_track(unified)
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        _ => return Err("Unsupported source for track info".to_string()),
    };
    
    // Cache the result
    set_cached(cache_key, serde_json::to_value(&track).unwrap());
    
    Ok(MonochromeResponse {
        data: Some(track),
        error: None,
    })
}

/// Get album info in Monochrome format
/// Endpoint: /album/{id} or /album?id={id}
pub async fn get_album_info(id: &str) -> Result<MonochromeResponse<MonochromeAlbum>, String> {
    ensure_clients_initialized().await;
    
    let (source, actual_id) = if id.contains(':') {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
    } else {
        (Some(MusicSource::Tidal), Some(id.to_string()))
    };
    
    let (source, actual_id) = match (source, actual_id) {
        (Some(s), Some(id)) => (s, id),
        _ => return Err("Invalid album ID format".to_string()),
    };
    
    let album = match source {
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_album(&actual_id).await?;
                convert_to_monochrome_album(unified)
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_album(&actual_id).await?;
                convert_to_monochrome_album(unified)
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_album(&actual_id).await?;
                convert_to_monochrome_album(unified)
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
    };
    
    Ok(MonochromeResponse {
        data: Some(album),
        error: None,
    })
}

/// Get artist info in Monochrome format
/// Endpoint: /artist/?id={id}
pub async fn get_artist_info(id: &str) -> Result<MonochromeResponse<MonochromeArtist>, String> {
    ensure_clients_initialized().await;
    
    let (source, actual_id) = if id.contains(':') {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
    } else {
        (Some(MusicSource::Tidal), Some(id.to_string()))
    };
    
    let (source, actual_id) = match (source, actual_id) {
        (Some(s), Some(id)) => (s, id),
        _ => return Err("Invalid artist ID format".to_string()),
    };
    
    let artist = match source {
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_artist(&actual_id).await?;
                convert_to_monochrome_artist(unified)
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_artist(&actual_id).await?;
                convert_to_monochrome_artist(unified)
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let unified = client.get_artist(&actual_id).await?;
                convert_to_monochrome_artist(unified)
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
    };
    
    Ok(MonochromeResponse {
        data: Some(artist),
        error: None,
    })
}

/// Get stream URL in Monochrome format with automatic quality selection
/// Endpoint: /stream?id={id}&quality={quality}
/// If quality is "auto" or not specified, will find the best available quality
/// Uses Lossless FLAC quality hierarchy: Dolby Atmos > Ultra Hi-Res > Hi-Res > Lossless > High > Normal > Low
pub async fn get_stream_url(id: &str, quality: &str) -> Result<MonochromeStreamInfo, String> {
    ensure_clients_initialized().await;
    
    // Parse quality (default to auto for best quality)
    let quality_level = if quality == "auto" || quality.is_empty() {
        None // Will auto-select best quality
    } else {
        Some(Quality::from_str(quality))
    };
    
    // Parse source from ID
    let (source, actual_id) = if id.contains(':') {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
    } else {
        (Some(MusicSource::Tidal), Some(id.to_string()))
    };
    
    let (source, actual_id) = match (source, actual_id) {
        (Some(s), Some(id)) => (s, id),
        _ => return Err("Invalid track ID format".to_string()),
    };
    
    // If quality is auto, try from highest to lowest (Lossless FLAC hierarchy)
    let qualities_to_try = if quality_level.is_none() {
        vec![
            Quality::DolbyAtmos,    // Highest priority - Dolby Atmos
            Quality::UltraHiRes,   // 24-bit, ≤192 kHz
            Quality::HiRes,        // 24-bit, ≤96 kHz
            Quality::High,         // 16-bit, 44.1 kHz (CD)
            Quality::Normal,       // 320 kbps
            Quality::Low,          // 128 kbps
        ]
    } else {
        vec![quality_level.unwrap()]
    };
    
    let mut last_error = String::new();
    
    for quality in qualities_to_try {
        let result = match source {
            MusicSource::Tidal => {
                let client = TIDAL_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await
                } else {
                    Err("Tidal client not available".to_string())
                }
            }
            MusicSource::Qobuz => {
                let client = QOBUZ_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await
                } else {
                    Err("Qobuz client not available".to_string())
                }
            }
            MusicSource::Deezer => {
                let client = DEEZER_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await
                } else {
                    Err("Deezer client not available".to_string())
                }
            }
            MusicSource::SoundCloud => {
                let client = SOUNDCLOUD_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await
                } else {
                    Err("SoundCloud client not available".to_string())
                }
            }
            _ => Err("Unsupported source".to_string()),
        };
        
        match result {
            Ok(stream_info) => {
                return Ok(MonochromeStreamInfo {
                    url: stream_info.url.clone(),
                    stream_url: Some(stream_info.url),
                    quality: stream_info.quality.as_str().to_string(),
                    codec: stream_info.codec,
                    bitrate: stream_info.bitrate,
                    sample_rate: stream_info.sample_rate,
                    bit_depth: stream_info.bit_depth,
                    encryption_key: stream_info.encryption_key,
                    source: stream_info.source.as_str().to_string(),
                });
            }
            Err(e) => {
                last_error = e;
                continue; // Try next quality
            }
        }
    }
    
    Err(format!("Failed to get stream URL at any quality. Last error: {}", last_error))
}

/// Search in Monochrome format (enhanced with parallel search)
/// Endpoint: /search?q={query}&type={type}
pub async fn search(query: &str, search_type: &str, limit: u32) -> Result<Value, String> {
    ensure_clients_initialized().await;
    
    use crate::client::{QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT};
    
    let mut results = Value::Object(serde_json::Map::new());
    
    // Parallel search across all services (advanced feature)
    let (qobuz_results, tidal_results, deezer_results) = tokio::join!(
        async {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                match search_type {
                    "tracks" => client.search_tracks(query, limit).await.ok(),
                    "albums" => client.search_albums(query, limit).await.ok(),
                    "artists" => client.search_artists(query, limit).await.ok(),
                    _ => None,
                }
            } else {
                None
            }
        },
        async {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                match search_type {
                    "tracks" => client.search_tracks(query, limit).await.ok(),
                    "albums" => client.search_albums(query, limit).await.ok(),
                    "artists" => client.search_artists(query, limit).await.ok(),
                    _ => None,
                }
            } else {
                None
            }
        },
        async {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                match search_type {
                    "tracks" => client.search_tracks(query, limit).await.ok(),
                    "albums" => client.search_albums(query, limit).await.ok(),
                    "artists" => client.search_artists(query, limit).await.ok(),
                    _ => None,
                }
            } else {
                None
            }
        }
    );
    
    // Combine and sort by quality (advanced feature)
    let mut all_items: Vec<Value> = Vec::new();
    
    if let Some(items) = qobuz_results {
        for item in items {
            all_items.push(serde_json::to_value(item).unwrap_or_default());
        }
    }
    if let Some(items) = tidal_results {
        for item in items {
            all_items.push(serde_json::to_value(item).unwrap_or_default());
        }
    }
    if let Some(items) = deezer_results {
        for item in items {
            all_items.push(serde_json::to_value(item).unwrap_or_default());
        }
    }
    
    // Sort by quality (advanced feature)
    all_items.sort_by(|a, b| {
        let quality_a = a.get("audio_quality").and_then(|q| q.as_str()).unwrap_or("");
        let quality_b = b.get("audio_quality").and_then(|q| q.as_str()).unwrap_or("");
        quality_b.cmp(quality_a) // Higher quality first
    });
    
    results["data"] = serde_json::json!(all_items);
    results["total"] = serde_json::json!(all_items.len());
    
    Ok(results)
}

/// Convert UnifiedTrack to MonochromeTrack
fn convert_to_monochrome_track(track: UnifiedTrack) -> MonochromeTrack {
    MonochromeTrack {
        id: track.id,
        title: track.title,
        version: track.version,
        duration: track.duration.unwrap_or(0),
        track_number: track.track_number,
        volume_number: None,
        explicit: track.explicit,
        audio_quality: track.audio_quality,
        audio_modes: None,
        stream_url: track.stream_url,
        preview_url: track.preview_url,
        copyright: track.copyright,
        url: track.url,
        artists: track.artists.into_iter().map(convert_to_monochrome_artist).collect(),
        album: convert_to_monochrome_album(track.album),
        extra: serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// Convert UnifiedArtist to MonochromeArtist
fn convert_to_monochrome_artist(artist: UnifiedArtist) -> MonochromeArtist {
    MonochromeArtist {
        id: artist.id,
        name: artist.name,
        picture: artist.picture,
        url: artist.url,
        extra: serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// Convert UnifiedAlbum to MonochromeAlbum
fn convert_to_monochrome_album(album: UnifiedAlbum) -> MonochromeAlbum {
    MonochromeAlbum {
        id: album.id,
        title: album.title,
        cover: album.cover,
        cover_big: album.cover_big,
        duration: album.duration,
        track_count: album.track_count,
        release_date: album.release_date,
        copyright: album.copyright,
        url: album.url,
        artist: album.artist.map(convert_to_monochrome_artist),
        extra: serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// Advanced search with quality and source filtering
pub async fn advanced_search(
    query: &str, 
    search_type: &str, 
    limit: u32,
    min_quality: Option<&str>,
    sources: Option<Vec<&str>>,
) -> Result<Value, String> {
    ensure_clients_initialized().await;
    
    let filter = match search_type {
        "tracks" => SearchFilter::Tracks,
        "albums" => SearchFilter::Albums,
        "artists" => SearchFilter::Artists,
        "playlists" => SearchFilter::Playlists,
        _ => SearchFilter::Tracks,
    };
    
    let min_quality_enum = min_quality.map(|q| Quality::from_str(q));
    
    let results = crate::client::advanced_search(
        query, 
        filter, 
        1, 
        min_quality_enum, 
        None
    ).await?;
    
    let filtered_items: Vec<serde_json::Value> = results.items.into_iter()
        .filter_map(|item| {
            // Filter by sources if specified
            if let Some(ref source_list) = sources {
                let source_str = match &item {
                    MediaItem::Track(track) => track.source.as_str().to_lowercase(),
                    MediaItem::Album(album) => album.source.as_str().to_lowercase(),
                    MediaItem::Artist(artist) => artist.source.as_str().to_lowercase(),
                    MediaItem::Playlist(playlist) => playlist.source.as_str().to_lowercase(),
                };
                if !source_list.iter().any(|s| s.to_lowercase() == source_str) {
                    return None;
                }
            }
            Some(serde_json::to_value(item).unwrap_or_default())
        })
        .take(limit as usize)
        .collect();
    
    let mut response = Value::Object(serde_json::Map::new());
    response["data"] = serde_json::json!(filtered_items);
    response["total"] = serde_json::json!(filtered_items.len());
    
    Ok(response)
}

/// Get home page data in Monochrome format
pub async fn get_home_data() -> Result<Value, String> {
    ensure_clients_initialized().await;
    
    let sections = crate::advanced_suggestions::fetch_home_sections().await?;
    
    let home_sections: Vec<serde_json::Value> = sections.into_iter()
        .map(|section| {
            serde_json::json!({
                "id": section.id,
                "title": section.title,
                "items": section.items.into_iter()
                    .map(|item| convert_media_item_to_json(item))
                    .collect::<Vec<_>>(),
                "section_type": format!("{:?}", section.section_type),
                "source": section.source.as_str(),
            })
        })
        .collect();
    
    let mut response = Value::Object(serde_json::Map::new());
    response["data"] = serde_json::json!(home_sections);
    response["total"] = serde_json::json!(home_sections.len());
    
    Ok(response)
}

/// Get search suggestions in Monochrome format
pub async fn get_search_suggestions(query: &str) -> Result<Value, String> {
    ensure_clients_initialized().await;
    
    let suggestions = crate::advanced_suggestions::fetch_search_suggestions(query).await?;
    
    let suggestions_json: Vec<serde_json::Value> = suggestions.into_iter()
        .map(|suggestion| {
            serde_json::json!({
                "text": suggestion.text,
                "suggestion_type": format!("{:?}", suggestion.suggestion_type),
                "metadata": suggestion.metadata.map(|meta| {
                    serde_json::json!({
                        "id": meta.id,
                        "artist": meta.artist,
                        "cover": meta.cover,
                        "source": meta.source.as_str(),
                    })
                }),
            })
        })
        .collect();
    
    let mut response = Value::Object(serde_json::Map::new());
    response["data"] = serde_json::json!(suggestions_json);
    response["total"] = serde_json::json!(suggestions_json.len());
    
    Ok(response)
}

/// Advanced search with Monochrome-style response format
pub async fn advanced_search(query: &str, filter: &str) -> Result<Value, String> {
    ensure_clients_initialized().await;
    
    let search_filter = match filter {
        "albums" => SearchFilter::Albums,
        "artists" => SearchFilter::Artists,
        "playlists" => SearchFilter::Playlists,
        _ => SearchFilter::Tracks,
    };
    
    let results = client::search(query, search_filter, 1).await?;
    
    let items_json: Vec<serde_json::Value> = results.items.into_iter()
        .map(|item| convert_media_item_to_json(item))
        .collect();
    
    let mut response = Value::Object(serde_json::Map::new());
    response["data"] = serde_json::json!(items_json);
    response["total"] = serde_json::json!(items_json.len());
    response["limit"] = serde_json::json!(results.limit);
    response["offset"] = serde_json::json!(results.offset);
    
    Ok(response)
}

fn convert_media_item_to_json(item: MediaItem) -> serde_json::Value {
    match item {
        MediaItem::Track(track) => serde_json::json!({
            "id": track.id,
            "title": track.title,
            "artist": track.artist.map(|a| a.name),
            "cover": track.album.and_then(|a| a.cover),
            "duration": track.duration,
            "type": "track",
            "audio_quality": track.audio_quality,
        }),
        MediaItem::Album(album) => serde_json::json!({
            "id": album.id,
            "title": album.title,
            "artist": album.artist.map(|a| a.name),
            "cover": album.cover,
            "duration": album.duration,
            "type": "album",
        }),
        MediaItem::Artist(artist) => serde_json::json!({
            "id": artist.id,
            "title": artist.name,
            "artist": Some(artist.name.clone()),
            "cover": artist.picture,
            "type": "artist",
        }),
        MediaItem::Playlist(playlist) => serde_json::json!({
            "id": playlist.id,
            "title": playlist.title,
            "artist": playlist.creator.map(|c| c.name),
            "cover": playlist.picture,
            "duration": playlist.duration,
            "type": "playlist",
        }),
    }
}
