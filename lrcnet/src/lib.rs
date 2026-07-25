mod mapper;
mod parser;
mod kugou;
mod betterlyrics;

use anyhow::Result;
use bex_core::lyrics::{
    types::{Lyrics, LyricsMatch, LyricsMetadata, TrackMetadata},
    ext::http,
    Guest,
};

const LRCLIB_BASE_URL: &str = "https://lrclib.net/api";

struct Component;

impl Guest for Component {
    fn get_lyrics(metadata: TrackMetadata) -> Result<Option<(Lyrics, LyricsMetadata)>, String> {
        // Try multiple sources in order of preference
        // 1. LRCLIB (primary - already has synced lyrics)
        // 2. KuGou (Chinese lyrics, good fallback)
        // 3. BetterLyrics (TTML format)
        
        // Try LRCLIB first
        if let Ok(Some(result)) = try_lrclib(&metadata) {
            return Ok(Some(result));
        }
        
        // Try KuGou
        if let Ok(Some(result)) = try_kugou(&metadata) {
            return Ok(Some(result));
        }
        
        // Try BetterLyrics
        if let Ok(Some(result)) = try_betterlyrics(&metadata) {
            return Ok(Some(result));
        }
        
        Ok(None)
    }

    fn search(query: String) -> Result<Vec<LyricsMatch>, String> {
        let url = format!(
            "{}/search?q={}",
            LRCLIB_BASE_URL,
            urlencoding::encode(&query)
        );

        let resp = http::get(&url)
            .header("User-Agent", "Void Music-BEX/0.1.0")
            .send()
            .map_err(|e| e.to_string())?;

        if resp.status != 200 {
            return Err(format!("Status {}", resp.status));
        }

        let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
        let results: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| e.to_string())?;
        Ok(results
            .into_iter()
            .map(|r| mapper::to_lyrics_match(&r))
            .collect())
    }

    fn get_lyrics_by_id(id: String) -> Result<(Lyrics, LyricsMetadata), String> {
        let url = format!("{}/get/{}", LRCLIB_BASE_URL, id);

        let resp = http::get(&url)
            .header("User-Agent", "Void Music-BEX/0.1.0")
            .send()
            .map_err(|e| e.to_string())?;

        if resp.status != 200 {
            return Err(format!("Status {}", resp.status));
        }

        let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        Ok(mapper::to_lyrics_and_metadata(&json))
    }
}

/// Try to fetch lyrics from LRCLIB
fn try_lrclib(metadata: &TrackMetadata) -> Result<Option<(Lyrics, LyricsMetadata)>, String> {
    let url = format!(
        "{}/get?artist_name={}&track_name={}&album_name={}&duration={}",
        LRCLIB_BASE_URL,
        urlencoding::encode(&metadata.artist),
        urlencoding::encode(&metadata.title),
        urlencoding::encode(metadata.album.as_deref().unwrap_or("")),
        metadata.duration_ms.unwrap_or(0) / 1000
    );

    let resp = http::get(&url)
        .header("User-Agent", "Void Music-BEX/0.1.0")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status == 404 {
        return Ok(None);
    }
    if resp.status != 200 {
        return Err(format!("LRCLIB Status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    
    let (lyrics, mut meta) = mapper::to_lyrics_and_metadata(&json);
    meta.source = Some("LRCLIB".to_string());
    Ok(Some((lyrics, meta)))
}

/// Try to fetch lyrics from KuGou
fn try_kugou(metadata: &TrackMetadata) -> Result<Option<(Lyrics, LyricsMetadata)>, String> {
    let keyword = kugou::build_keyword(&metadata.artist, &metadata.title);
    let duration_sec = metadata.duration_ms.map(|d| d / 1000);
    
    // Try searching by song first for better matching
    if let Ok(songs) = kugou::search_songs(&keyword) {
        for song in songs {
            if let Some(song_duration) = song.duration {
                if let Some(track_duration) = duration_sec {
                    // Check if duration matches within tolerance (8 seconds)
                    if (song_duration - track_duration as i64).abs() <= 8 {
                        if let Some(ref hash) = song.hash {
                            if let Ok(candidates) = kugou::search_by_hash(hash) {
                                if let Some(candidate) = candidates.first() {
                                    if let Ok(lyrics_text) = kugou::download_lyrics(candidate.id, &candidate.access_key) {
                                        let normalized = kugou::normalize_lyrics(&lyrics_text);
                                        let lines = parser::parse_lrc(&normalized);
                                        
                                        let lyrics = Lyrics {
                                            plain: Some(normalized.clone()),
                                            lrc: Some(normalized),
                                            lines: Some(lines),
                                            is_instrumental: false,
                                            sync_type: bex_core::lyrics::types::LyricsSyncType::Line,
                                        };
                                        
                                        let meta = LyricsMetadata {
                                            author: None,
                                            source: Some("KuGou".to_string()),
                                            language: None,
                                            copyright: None,
                                            is_verified: false,
                                        };
                                        
                                        return Ok(Some((lyrics, meta)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback to keyword search
    if let Ok(candidates) = kugou::search_by_keyword(&keyword) {
        if let Some(candidate) = candidates.first() {
            if let Ok(lyrics_text) = kugou::download_lyrics(candidate.id, &candidate.access_key) {
                let normalized = kugou::normalize_lyrics(&lyrics_text);
                let lines = parser::parse_lrc(&normalized);
                
                let lyrics = Lyrics {
                    plain: Some(normalized.clone()),
                    lrc: Some(normalized),
                    lines: Some(lines),
                    is_instrumental: false,
                    sync_type: bex_core::lyrics::types::LyricsSyncType::Line,
                };
                
                let meta = LyricsMetadata {
                    author: None,
                    source: Some("KuGou".to_string()),
                    language: None,
                    copyright: None,
                    is_verified: false,
                };
                
                return Ok(Some((lyrics, meta)));
            }
        }
    }
    
    Ok(None)
}

/// Try to fetch lyrics from BetterLyrics
fn try_betterlyrics(metadata: &TrackMetadata) -> Result<Option<(Lyrics, LyricsMetadata)>, String> {
    let duration_sec = metadata.duration_ms.map(|d| d as i64 / 1000);
    let album = metadata.album.as_deref();
    
    if let Ok(ttml) = betterlyrics::fetch_lyrics(
        &metadata.artist,
        &metadata.title,
        duration_sec,
        album,
    ) {
        if let Ok(lrc) = betterlyrics::ttml_to_lrc(&ttml) {
            let lines = parser::parse_lrc(&lrc);
            
            let lyrics = Lyrics {
                plain: Some(lrc.clone()),
                lrc: Some(lrc),
                lines: Some(lines),
                is_instrumental: false,
                sync_type: bex_core::lyrics::types::LyricsSyncType::Line,
            };
            
            let meta = LyricsMetadata {
                author: None,
                source: Some("BetterLyrics".to_string()),
                language: None,
                copyright: None,
                is_verified: false,
            };
            
            return Ok(Some((lyrics, meta)));
        }
    }
    
    Ok(None)
}

bex_core::export_lyrics!(Component);
