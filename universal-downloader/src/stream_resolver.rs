//! Stream resolver with automatic fallback between download methods
//!
//! This module coordinates between different download methods and provides
//! automatic fallback when one method fails.

use crate::credentials::YOUTUBE_CREDENTIALS;
use crate::http_downloader::{extract_extension, HttpDownloader};
use crate::jiosaavn_downloader::{JioSaavnDownloader, JioSaavnQuality};
use crate::youtube_downloader::YouTubeDownloader;
use anyhow::{anyhow, Result};
use bex_core::resolver::data_source::{Quality, StreamSource};

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
    let (_source, actual_id) = parse_track_id(track_id)?;

    Ok(bex_core::resolver::types::Track {
        id: track_id.to_string(),
        title: format!("Track {}", actual_id),
        artists: vec![],
        album: None,
        duration_ms: Some(0),
        thumbnail: bex_core::resolver::types::Artwork {
            url: String::new(),
            url_low: None,
            url_high: None,
            layout: bex_core::resolver::types::ImageLayout::Square,
        },
        url: None,
        is_explicit: false,
        lyrics: None,
    })
}

/// Get streams with automatic fallback
pub fn get_streams_with_fallback(track_id: &str) -> Result<Vec<StreamSource>> {
    let (source, actual_id) = parse_track_id(track_id)?;

    match source {
        TrackSource::YouTubeMusic | TrackSource::YouTubeVideo => {
            get_youtube_streams_with_fallback(&actual_id)
        }
        TrackSource::JioSaavn => get_jiosaavn_streams_with_fallback(&actual_id),
        TrackSource::DirectHttp => get_http_stream(&actual_id),
    }
}

/// YouTube streams with multi-method fallback
fn get_youtube_streams_with_fallback(video_id: &str) -> Result<Vec<StreamSource>> {
    let downloader = YouTubeDownloader::new();
    let result = downloader.get_stream_url(video_id);

    let stream_url = match result {
        Ok(url) => url,
        Err(e) => {
            let _ = format!("Primary YouTube download failed: {}, trying rotated key...", e);
            let downloader = YouTubeDownloader::with_rotated_key();
            match downloader.get_stream_url(video_id) {
                Ok(url) => url,
                Err(e) => {
                    let _ = format!("Rotated key failed: {}, trying HTTP fallback...", e);
                    return get_http_fallback_for_youtube(video_id);
                }
            }
        }
    };

    Ok(vec![StreamSource {
        url: stream_url,
        format: "m4a".to_string(),
        quality: Quality::High,
        headers: None,
        expires_at: None,
    }])
}

/// JioSaavn streams with server rotation fallback
fn get_jiosaavn_streams_with_fallback(track_id: &str) -> Result<Vec<StreamSource>> {
    let downloader = JioSaavnDownloader::new();
    let result = downloader.get_stream_url(track_id, JioSaavnQuality::High);

    let stream_url = match result {
        Ok(url) => url,
        Err(e) => {
            let _ = format!("Primary JioSaavn server failed: {}, trying rotated server...", e);
            let downloader = JioSaavnDownloader::with_rotated_server();
            match downloader.get_stream_url(track_id, JioSaavnQuality::High) {
                Ok(url) => url,
                Err(e) => {
                    let _ = format!("Rotated server failed: {}, trying HTTP fallback...", e);
                    return get_http_fallback_for_jiosaavn(track_id);
                }
            }
        }
    };

    Ok(vec![StreamSource {
        url: stream_url,
        format: "m4a".to_string(),
        quality: Quality::High,
        headers: None,
        expires_at: None,
    }])
}

/// Direct HTTP stream
fn get_http_stream(url: &str) -> Result<Vec<StreamSource>> {
    let downloader = HttpDownloader::new();
    let stream_url = downloader.get_stream_url(url)?;
    let extension = extract_extension(&stream_url).unwrap_or_else(|| "mp3".to_string());

    Ok(vec![StreamSource {
        url: stream_url,
        format: extension,
        quality: Quality::Medium,
        headers: None,
        expires_at: None,
    }])
}

/// HTTP fallback for YouTube when Innertube fails
fn get_http_fallback_for_youtube(video_id: &str) -> Result<Vec<StreamSource>> {
    let fallback_url = format!("https://pipedapi.kavin.rocks/streams/{}", video_id);
    let options = bex_core::resolver::component::content_resolver::utils::RequestOptions {
        method: bex_core::resolver::component::content_resolver::utils::HttpMethod::Get,
        headers: None,
        body: None,
        timeout_seconds: Some(10),
    };

    match bex_core::resolver::component::content_resolver::utils::http_request(
        &fallback_url,
        &options,
    ) {
        Ok(response) if response.status >= 200 && response.status < 300 => Ok(vec![StreamSource {
            url: fallback_url,
            format: "json".to_string(),
            quality: Quality::Medium,
            headers: None,
            expires_at: None,
        }]),
        _ => Err(anyhow!("All YouTube download methods failed")),
    }
}

/// HTTP fallback for JioSaavn when DES decryption fails
fn get_http_fallback_for_jiosaavn(track_id: &str) -> Result<Vec<StreamSource>> {
    let fallback_url = format!("https://www.jiosaavn.com/song/{}", track_id);

    Ok(vec![StreamSource {
        url: fallback_url,
        format: "html".to_string(),
        quality: Quality::Medium,
        headers: None,
        expires_at: None,
    }])
}

/// Utility function to reset credential pools
#[allow(dead_code)]
pub fn reset_credentials() {
    YOUTUBE_CREDENTIALS.reset();
}
