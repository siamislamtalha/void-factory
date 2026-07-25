#![allow(dead_code)]
#![allow(unused_variables)]

//! Multi-Source Music Aggregator — bex-core content-resolver plugin.
//!
//! This plugin aggregates search results from multiple music services with source tagging.
//! It uses the actual API implementations from existing plugins:
//! - YouTube Music (via Innertube API with backup API keys)
//! - YouTube Video (via Innertube API)
//! - JioSaavn (via public API with DES decryption)
//!
//! All results are tagged with source prefixes for identification:
//! - ytm: for YouTube Music
//! - ytv: for YouTube Video  
//! - jio: for JioSaavn
//!
//! Streaming and download functionality delegates to the actual service implementations.

// Credential pool
mod credentials;

// YouTube Music modules
mod ytmusic_client;
mod ytmusic_cipher;
mod ytmusic_parser;
mod ytmusic_mapper;

// YouTube Video modules
mod ytvideo_client;
mod ytvideo_cipher;
mod ytvideo_parser;
mod ytvideo_mapper;

// JioSaavn modules
mod jiosaavn_client;
mod jiosaavn_crypto;
mod jiosaavn_mapper;
mod jiosaavn_types;

use bex_core::resolver::{
    data_source::{
        AlbumDetails, ArtistDetails, Guest as DataSourceGuest,
        PagedAlbums, PagedMediaItems, PagedTracks, PlaylistDetails,
        SearchFilter, StreamSource,
    },
    discovery::{Guest as DiscoveryGuest, Section},
    types::MediaItem,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MusicSource {
    YouTubeMusic,
    YouTubeVideo,
    JioSaavn,
}

impl MusicSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::YouTubeMusic => "YouTube Music",
            Self::YouTubeVideo => "YouTube Video",
            Self::JioSaavn => "JioSaavn",
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            Self::YouTubeMusic => "ytm:",
            Self::YouTubeVideo => "ytv:",
            Self::JioSaavn => "jio:",
        }
    }
}

struct Component;

impl DiscoveryGuest for Component {
    fn get_home_sections() -> Result<Vec<Section>, String> {
        // Aggregate home sections from YouTube Music and JioSaavn
        let mut all_sections = Vec::new();
        
        // Try YouTube Music
        if let Ok(sections) = ytmusic_client::fetch_home_data() {
            all_sections.extend(sections);
        }
        
        // Try JioSaavn (if it has home sections)
        // Note: JioSaavn may not have home sections in the same format
        
        Ok(all_sections)
    }

    fn load_more(section_id: String, page_token: String) -> Result<Vec<MediaItem>, String> {
        // Parse section_id to determine source
        if section_id.starts_with("ytm:") {
            ytmusic_client::load_more_items(&section_id[4..], &page_token)
                .map_err(|e| e.to_string())
        } else if section_id.starts_with("jio:") {
            // JioSaavn load more if available
            Ok(vec![])
        } else {
            ytmusic_client::load_more_items(&section_id, &page_token)
                .map_err(|e| e.to_string())
        }
    }
}

impl DataSourceGuest for Component {
    fn get_track_details(id: String) -> Result<bex_core::resolver::types::Track, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_track_details(&actual_id)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_track_details(&actual_id)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_track_details(&actual_id)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn get_album_details(id: String) -> Result<AlbumDetails, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_album_details(&actual_id)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_album_details(&actual_id)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_album_details(&actual_id)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn more_album_tracks(id: String, page_token: String) -> Result<PagedTracks, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::more_album_tracks(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::more_album_tracks(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::more_album_tracks(&actual_id, &page_token)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn get_artist_details(id: String) -> Result<ArtistDetails, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_artist_details(&actual_id)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_artist_details(&actual_id)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_artist_details(&actual_id)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn more_artist_albums(id: String, page_token: String) -> Result<PagedAlbums, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::more_artist_albums(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::more_artist_albums(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::more_artist_albums(&actual_id, &page_token)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn get_playlist_details(id: String) -> Result<PlaylistDetails, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_playlist_details(&actual_id)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_playlist_details(&actual_id)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_playlist_details(&actual_id)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn more_playlist_tracks(id: String, page_token: String) -> Result<PagedTracks, String> {
        let (source, actual_id) = parse_source_id(&id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::more_playlist_tracks(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::more_playlist_tracks(&actual_id, &page_token)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::more_playlist_tracks(&actual_id, &page_token)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn get_streams(track_id: String) -> Result<Vec<StreamSource>, String> {
        let (source, actual_id) = parse_source_id(&track_id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_streams(&actual_id)
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_streams(&actual_id)
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_streams(&actual_id)
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn get_segments(_track_id: String) -> Result<Vec<bex_core::resolver::types::MediaSegment>, String> {
        Ok(vec![])
    }

    fn get_radio_tracks(
        reference_id: String,
        page_token: Option<String>,
    ) -> Result<PagedTracks, String> {
        let (source, actual_id) = parse_source_id(&reference_id)?;
        
        match source {
            MusicSource::YouTubeMusic => ytmusic_client::get_radio_tracks(&actual_id, page_token.as_deref())
                .map_err(|e| format!("YouTube Music error: {}", e)),
            MusicSource::YouTubeVideo => ytvideo_client::get_radio_tracks(&actual_id, page_token.as_deref())
                .map_err(|e| format!("YouTube Video error: {}", e)),
            MusicSource::JioSaavn => jiosaavn_client::get_radio_tracks(&actual_id, page_token.as_deref())
                .map_err(|e| format!("JioSaavn error: {}", e)),
        }
    }

    fn search(
        query: String,
        filter: SearchFilter,
        page_token: Option<String>,
    ) -> Result<PagedMediaItems, String> {
        // Multi-source search: query all services and aggregate results
        let mut all_items = Vec::new();
        
        // Search YouTube Music (with Innertube API and backup keys)
        if let Ok(results) = ytmusic_client::search(&query, filter, page_token.as_deref()) {
            all_items.extend(results.items.into_iter().map(|mut item| {
                add_source_tag(&mut item, MusicSource::YouTubeMusic);
                item
            }));
        }
        
        // Search YouTube Video
        if let Ok(results) = ytvideo_client::search(&query, filter, page_token.as_deref()) {
            all_items.extend(results.items.into_iter().map(|mut item| {
                add_source_tag(&mut item, MusicSource::YouTubeVideo);
                item
            }));
        }
        
        // Search JioSaavn (with DES decryption support)
        let page = page_token.and_then(|t| t.parse::<i32>().ok()).unwrap_or(1);
        if let Ok(results) = jiosaavn_client::search(&query, filter, page) {
            all_items.extend(results.items.into_iter().map(|mut item| {
                add_source_tag(&mut item, MusicSource::JioSaavn);
                item
            }));
        }
        
        Ok(PagedMediaItems {
            items: all_items,
            next_page_token: None, // Aggregated search doesn't support pagination
        })
    }
}

fn parse_source_id(id: &str) -> Result<(MusicSource, String), String> {
    if let Some(prefix) = id.strip_prefix("ytm:") {
        Ok((MusicSource::YouTubeMusic, prefix.to_string()))
    } else if let Some(prefix) = id.strip_prefix("ytv:") {
        Ok((MusicSource::YouTubeVideo, prefix.to_string()))
    } else if let Some(prefix) = id.strip_prefix("jio:") {
        Ok((MusicSource::JioSaavn, prefix.to_string()))
    } else {
        // Default to YouTube Music if no prefix
        Ok((MusicSource::YouTubeMusic, id.to_string()))
    }
}

fn add_source_tag(item: &mut MediaItem, source: MusicSource) {
    // Add source prefix to ID for identification
    match item {
        MediaItem::Track(track) => {
            if !track.id.contains(':') {
                track.id = format!("{}{}", source.id_prefix(), track.id);
            }
        }
        MediaItem::Album(album) => {
            if !album.id.contains(':') {
                album.id = format!("{}{}", source.id_prefix(), album.id);
            }
        }
        MediaItem::Artist(artist) => {
            if !artist.id.contains(':') {
                artist.id = format!("{}{}", source.id_prefix(), artist.id);
            }
        }
        MediaItem::Playlist(playlist) => {
            if !playlist.id.contains(':') {
                playlist.id = format!("{}{}", source.id_prefix(), playlist.id);
            }
        }
    }
}

bex_core::export_resolver!(Component);
