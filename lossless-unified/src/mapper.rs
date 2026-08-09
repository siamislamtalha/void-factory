use crate::types::*;
use bex_core::resolver::{
    types::{MediaItem as BexMediaItem, Track as BexTrack},
    data_source::StreamSource,
};

/// Convert Unified Track to BEX Track
pub fn map_track(track: UnifiedTrack) -> BexTrack {
    BexTrack {
        id: format!("{}{}", track.source.id_prefix(), track.id),
        title: track.title,
        artists: track.artists.unwrap_or_default().into_iter().map(|a| bex_core::resolver::types::ArtistSummary {
            id: format!("{}:{}", track.source.as_str(), a.id),
            name: a.name,
            thumbnail: a.picture.map(|p| bex_core::resolver::types::Artwork { url: p }),
            subtitle: None,
            url: a.url,
        }).collect(),
        album: track.album.as_ref().map(|a| bex_core::resolver::types::AlbumSummary {
            id: format!("{}:{}", track.source.as_str(), a.id),
            title: a.title.clone(),
            artists: a.artist.iter().map(|ar| bex_core::resolver::types::ArtistSummary {
                id: format!("{}:{}", track.source.as_str(), ar.id),
                name: ar.name.clone(),
                thumbnail: ar.picture.clone().map(|p| bex_core::resolver::types::Artwork { url: p }),
                subtitle: None,
                url: ar.url,
            }).collect(),
            thumbnail: a.cover.clone().map(|c| bex_core::resolver::types::Artwork { url: c }),
            subtitle: a.release_date.clone(),
            year: a.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
            url: a.url,
        }),
        duration_ms: Some(track.duration as u64 * 1000),
        thumbnail: track.album.as_ref().and_then(|a| a.cover.clone()).map(|c| bex_core::resolver::types::Artwork { url: c }),
        url: None,
        lyrics: None,
        is_explicit: false,
    }
}

/// Map MediaItem for home sections
pub fn map_media_item(item: MediaItemData) -> bex_core::resolver::types::MediaItem {
    match item {
        MediaItemData::Track(track) => {
            bex_core::resolver::types::MediaItem::Track(map_track(track))
        }
        MediaItemData::Album(_) => {
            // Albums not supported in BEX types
            bex_core::resolver::types::MediaItem::Track(BexTrack {
                id: "placeholder".to_string(),
                title: "Unsupported".to_string(),
                artists: vec![],
                album: None,
                duration_ms: Some(0),
                thumbnail: bex_core::resolver::types::Artwork { url: String::new() },
                url: None,
                lyrics: None,
                is_explicit: false,
            })
        }
        MediaItemData::Artist(_) => {
            // Artists not supported in BEX types
            bex_core::resolver::types::MediaItem::Track(BexTrack {
                id: "placeholder".to_string(),
                title: "Unsupported".to_string(),
                artists: vec![],
                album: None,
                duration_ms: Some(0),
                thumbnail: bex_core::resolver::types::Artwork { url: String::new() },
                url: None,
                lyrics: None,
                is_explicit: false,
            })
        }
        MediaItemData::Playlist(_) => {
            // Playlists not supported in BEX types
            bex_core::resolver::types::MediaItem::Track(BexTrack {
                id: "placeholder".to_string(),
                title: "Unsupported".to_string(),
                artists: vec![],
                album: None,
                duration_ms: Some(0),
                thumbnail: bex_core::resolver::types::Artwork { url: String::new() },
                url: None,
                lyrics: None,
                is_explicit: false,
            })
        }
    }
}

/// Parse source prefix from ID
pub fn parse_source_id(id: &str) -> Result<(MusicSource, String), String> {
    if let Some((source, actual_id)) = id.split_once(':') {
        let source = MusicSource::from_str(source)
            .ok_or_else(|| format!("Unknown source: {}", source))?;
        Ok((source, actual_id.to_string()))
    } else {
        // Default to Tidal for Monochrome compatibility
        Ok((MusicSource::Tidal, id.to_string()))
    }
}

/// Convert BEX MediaItem to Unified types (for internal use)
pub fn media_item_to_unified(item: &BexMediaItem) -> Option<MediaItemData> {
    match item {
        BexMediaItem::Track(track) => {
            let (source, actual_id) = parse_source_id(&track.id).ok()?;
            Some(MediaItemData::Track(UnifiedTrack {
                id: actual_id,
                title: track.title.clone(),
                duration: track.duration_ms.map(|d| (d / 1000) as u32).unwrap_or(0),
                track_number: None,
                volume_number: None,
                replay_gain: None,
                peak: None,
                available: true,
                audio_quality: None,
                audio_modes: None,
                artist: track.artists.first().map(|a| UnifiedArtist {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    picture: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    url: a.url.clone(),
                    source,
                }),
                artists: Some(track.artists.iter().map(|a| UnifiedArtist {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    picture: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    url: a.url.clone(),
                    source,
                }).collect()),
                album: track.album.as_ref().map(|a| UnifiedAlbum {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    cover: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    duration: None,
                    track_count: None,
                    release_date: a.subtitle.clone(),
                    artist: None,
                    artists: None,
                    url: a.url.clone(),
                    source,
                }),
                source,
                qualities_available: vec![Quality::LosslessFlac, Quality::HiRes],
            }))
        }
        _ => None,
    }
}