//! Stream resolver with automatic fallback between download methods
//!
//! This module coordinates between different download methods and provides
//! automatic fallback when one method fails.

use crate::credentials::YOUTUBE_CREDENTIALS;
use crate::http_downloader::{extract_extension, HttpDownloader};
use crate::jiosaavn_downloader::{JioSaavnDownloader, JioSaavnQuality};
use crate::youtube_downloader::YouTubeDownloader;
use anyhow::{anyhow, Result};
use bex_core::resolver::data_source::StreamSource;

/// Parse track ID to determine source and actual ID
fn parse_track_id(track_id: &str) -> Result<(TrackSource, String)> {
    if track_id.starts_with("ytm:") {
        Ok((TrackSource::YouTubeMusic, track_id[4..].to_string()))
    } else if track_id.starts_with("ytv:") {
        Ok((TrackSource::YouTubeVideo, track_id[4..].to_string()))
    } else if track_id.starts_with("jio:") {
        Ok((TrackSource::JioSaavn, track_id[4..].to_string()))
    } else if track_id.starts_with("http://") || track_id.starts_with("https://") {
        Ok((TrackSource::DirectHttp, track_id.to_string()))
    } else {
        // Default to YouTube Music if no prefix
        Ok((TrackSource::YouTubeMusic, track_id.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackSource {
    YouTubeMusic,
    YouTubeVideo,
    JioSaavn,
    DirectHttp,
}

/// Get track details (basic implementation)
pub fn get_track_details(track_id: &str) -> Result<bex_core::resolver::types::Track> {
    let (source, actual_id) = parse_track_id(track_id)?;
    
    // For now, return basic track info
    // In a full implementation, you would fetch metadata from the source
    Ok(bex_core::resolver::types::Track {
        id: track_id.to_string(),
        title: format!("Track {}", actual_id),
        artist: vec!["Unknown Artist".to_string()],
        album: None,
        duration: Some(0),
        artists: None,
        album_artists: None,
        year: None,
        explicit: None,
        play_count: None,
        genre: None,
    })
}

/// Get streams with automatic fallback
pub fn get_streams_with_fallback(track_id: &str) -> Result<Vec<StreamSource>> {
    let (source, actual_id) = parse_track_id(track_id)?;
    
    match source {
        TrackSource::YouTubeMusic | TrackSource::YouTubeVideo => {
            get_youtube_streams_with_fallback(&actual_id)
        }
        TrackSource::JioSaavn => {
            get_jiosaavn_streams_with_fallback(&actual_id)
        }
        TrackSource::DirectHttp => {
            get_http_stream(&actual_id)
        }
    }
}

/// YouTube streams with multi-method fallback
fn get_youtube_streams_with_fallback(video_id: &str) -> Result<Vec<StreamSource>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    // Try YouTube downloader with current key
    let result = rt.block_on(async {
        let downloader = YouTubeDownloader::new();
        downloader.get_stream_url(video_id).await
    });
    
    let stream_url = match result {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Primary YouTube download failed: {}, trying rotated key...", e);
            
            // Try with rotated key
            let result = rt.block_on(async {
                let downloader = YouTubeDownloader::with_rotated_key();
                downloader.get_stream_url(video_id).await
            });
            
            match result {
                Ok(url) => url,
                Err(e) => {
                    eprintln!("Rotated key failed: {}, trying HTTP fallback...", e);
                    
                    // Final fallback to HTTP if YouTube fails completely
                    return get_http_fallback_for_youtube(video_id);
                }
            }
        }
    };
    
    Ok(vec![StreamSource {
        url: stream_url,
        format: "m4a".to_string(),
        quality: "high".to_string(),
        bitrate: Some(320),
        is_hls: false,
        is_dash: false,
        headers: None,
    }])
}

/// JioSaavn streams with server rotation fallback
fn get_jiosaavn_streams_with_fallback(track_id: &str) -> Result<Vec<StreamSource>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    // Try JioSaavn downloader with current server
    let result = rt.block_on(async {
        let downloader = JioSaavnDownloader::new();
        downloader.get_stream_url(track_id, JioSaavnQuality::High).await
    });
    
    let stream_url = match result {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Primary JioSaavn server failed: {}, trying rotated server...", e);
            
            // Try with rotated server
            let result = rt.block_on(async {
                let downloader = JioSaavnDownloader::with_rotated_server();
                downloader.get_stream_url(track_id, JioSaavnQuality::High).await
            });
            
            match result {
                Ok(url) => url,
                Err(e) => {
                    eprintln!("Rotated server failed: {}, trying HTTP fallback...", e);
                    
                    // Fallback to HTTP if JioSaavn fails
                    return get_http_fallback_for_jiosaavn(track_id);
                }
            }
        }
    };
    
    Ok(vec![StreamSource {
        url: stream_url,
        format: "m4a".to_string(),
        quality: "high".to_string(),
        bitrate: Some(320),
        is_hls: false,
        is_dash: false,
        headers: None,
    }])
}

/// Direct HTTP stream
fn get_http_stream(url: &str) -> Result<Vec<StreamSource>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    let result = rt.block_on(async {
        let downloader = HttpDownloader::new();
        downloader.get_stream_url(url).await
    });
    
    let stream_url = result?;
    let extension = extract_extension(&stream_url).unwrap_or("mp3".to_string());
    
    Ok(vec![StreamSource {
        url: stream_url,
        format: extension,
        quality: "unknown".to_string(),
        bitrate: None,
        is_hls: false,
        is_dash: false,
        headers: None,
    }])
}

/// HTTP fallback for YouTube when Innertube fails
fn get_http_fallback_for_youtube(video_id: &str) -> Result<Vec<StreamSource>> {
    // Use alternative YouTube streaming services as fallback
    // This is a simplified implementation - in production you might use:
    // - Piped API
    // - Invidious instances
    // - Other YouTube proxy services
    
    let fallback_url = format!("https://pipedapi.kavin.rocks/streams/{}", video_id);
    
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        let client = reqwest::Client::new();
        client.get(&fallback_url).send().await
    });
    
    match result {
        Ok(response) if response.status().is_success() => {
            // Parse Piped API response to get audio stream
            // For now, return the API URL as a fallback
            Ok(vec![StreamSource {
                url: fallback_url,
                format: "json".to_string(),
                quality: "fallback".to_string(),
                bitrate: None,
                is_hls: false,
                is_dash: false,
                headers: None,
            }])
        }
        _ => {
            Err(anyhow!("All YouTube download methods failed"))
        }
    }
}

/// HTTP fallback for JioSaavn when DES decryption fails
fn get_http_fallback_for_jiosaavn(track_id: &str) -> Result<Vec<StreamSource>> {
    // Try to construct a direct URL from the track ID
    // This is a simplified fallback implementation
    
    let fallback_url = format!("https://www.jiosaavn.com/song/{}", track_id);
    
    Ok(vec![StreamSource {
        url: fallback_url,
        format: "html".to_string(),
        quality: "fallback".to_string(),
        bitrate: None,
        is_hls: false,
        is_dash: false,
        headers: None,
    }])
}

/// Utility function to reset credential pools
pub fn reset_credentials() {
    YOUTUBE_CREDENTIALS.reset();
}
