use crate::types::*;
use bex_core::resolver::{
    types::{MediaItem as BexMediaItem, Track as BexTrack, Album as BexAlbum, Artist as BexArtist, Playlist as BexPlaylist},
    data_source::StreamSource,
};

/// Convert Unified Track to BEX Track
pub fn map_track(track: UnifiedTrack) -> BexTrack {
    BexTrack {
        id: format!("{}{}", track.source.id_prefix(), track.id),
        title: track.title,
        artist: track.artist.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
        artist_id: track.artist.as_ref().map(|a| a.id.clone()),
        album: track.album.as_ref().map(|a| a.title.clone()).unwrap_or_default(),
        album_id: track.album.as_ref().map(|a| a.id.clone()),
        duration: track.duration as i64,
        cover_art: track.album.as_ref().and_then(|a| a.cover.clone()),
        year: track.album.as_ref().and_then(|a| a.release_date.clone()).map(|d| {
            d.split('-').next().and_then(|y| y.parse().ok()).unwrap_or(0)
        }),
        genre: None,
        track_number: track.track_number.map(|n| n as i32),
        disc_number: track.volume_number.map(|n| n as i32),
        is_explicit: None,
    }
}

/// Convert Unified Artist to Bex Artist
pub fn map_artist(artist: UnifiedArtist) -> BexArtist {
    let prefixed_id = format!("{}:{}", artist.source.as_str(), artist.id);
    
    BexArtist {
        id: prefixed_id,
        name: artist.name,
        cover_art: artist.picture,
    }
}

/// Convert Unified Album to Bex Album
pub fn map_album(album: UnifiedAlbum) -> BexAlbum {
    let prefixed_id = format!("{}:{}", album.source.as_str(), album.id);
    
    BexAlbum {
        id: prefixed_id,
        title: album.title,
        artists: album.artist.iter().map(map_artist).collect(),
        cover_art: album.cover,
        track_count: album.track_count,
        release_date: album.release_date,
        duration: album.duration,
    }
}

/// Convert Unified Playlist to Bex Playlist
pub fn map_playlist(playlist: UnifiedPlaylist) -> BexPlaylist {
    let prefixed_id = format!("{}:{}", playlist.source.as_str(), playlist.id);
    
    BexPlaylist {
        id: prefixed_id,
        title: playlist.title,
        description: playlist.description,
        cover_art: playlist.cover,
        author: playlist.author,
        track_count: playlist.track_count,
        duration: playlist.duration,
    }
}

/// Map MediaItem for home sections
pub fn map_media_item(item: MediaItemData) -> bex_core::resolver::types::MediaItem {
    match item {
        MediaItemData::Track(track) => {
            bex_core::resolver::types::MediaItem::Track(map_track(track))
        }
        MediaItemData::Album(album) => {
            bex_core::resolver::types::MediaItem::Album(map_album(album))
        }
        MediaItemData::Artist(artist) => {
            bex_core::resolver::types::MediaItem::Artist(map_artist(artist))
        }
        MediaItemData::Playlist(playlist) => {
            bex_core::resolver::types::MediaItem::Playlist(map_playlist(playlist))
        }
    }
}

/// Convert MonochromeTrack to UnifiedTrack (for internal use)
pub fn monochrome_to_unified_track(track: MonochromeTrack, source: MusicSource) -> UnifiedTrack {
    UnifiedTrack {
        id: track.id,
        source,
        title: track.title,
        version: track.version,
        artists: track.artists.into_iter().map(|a| monochrome_to_unified_artist(a, source.clone())).collect(),
        album: monochrome_to_unified_album(track.album, source),
        duration: Some(track.duration),
        track_number: track.track_number,
        explicit: track.explicit,
        quality: track.audio_quality.as_ref().and_then(|q| Quality::from_str(q)),
        audio_quality: track.audio_quality,
        stream_url: track.stream_url,
        preview_url: track.preview_url,
        copyright: track.copyright,
        url: track.url,
    }
}

/// Convert MonochromeArtist to UnifiedArtist (for internal use)
pub fn monochrome_to_unified_artist(artist: MonochromeArtist, source: MusicSource) -> UnifiedArtist {
    UnifiedArtist {
        id: artist.id,
        source,
        name: artist.name,
        picture: artist.picture,
        url: artist.url,
    }
}

/// Convert MonochromeAlbum to UnifiedAlbum (for internal use)
pub fn monochrome_to_unified_album(album: MonochromeAlbum, source: MusicSource) -> UnifiedAlbum {
    UnifiedAlbum {
        id: album.id,
        source,
        title: album.title,
        cover: album.cover,
        cover_big: album.cover_big,
        duration: album.duration,
        track_count: album.track_count,
        release_date: album.release_date,
        copyright: album.copyright,
        url: album.url,
        artist: album.artist.map(|a| monochrome_to_unified_artist(a, source)),
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
                duration: track.duration as u32,
                track_number: track.track_number.map(|n| n as u32),
                volume_number: track.disc_number.map(|n| n as u32),
                replay_gain: track.replay_gain.as_ref().map(|rg| ReplayGain {
                    track_gain: rg.track_gain,
                    album_gain: rg.album_gain,
                    track_peak: rg.track_peak,
                    album_peak: rg.album_peak,
                }),
                peak: track.peak,
                available: track.available.unwrap_or(true),
                audio_quality: track.audio_quality.clone(),
                audio_modes: None,
                artist: Some(UnifiedArtist {
                    id: track.artist_id.clone().unwrap_or_default(),
                    name: track.artist.clone(),
                    picture: track.cover_art.clone(),
                    url: None,
                    source,
                }),
                artists: None,
                album: Some(UnifiedAlbum {
                    id: track.album_id.clone().unwrap_or_default(),
                    title: track.album.clone(),
                    cover: track.cover_art.clone(),
                    duration: None,
                    track_count: None,
                    release_date: None,
                    artist: None,
                    artists: None,
                    url: None,
                    source,
                }),
                source,
                qualities_available: vec![Quality::High, Quality::HiRes],
            }))
        }
        _ => None,
    }
}

/// Parse source prefix from ID
fn parse_source_id(id: &str) -> Result<(MusicSource, String), String> {
    if let Some(prefix) = id.strip_prefix("qobuz:") {
        Ok((MusicSource::Qobuz, prefix.to_string()))
    } else if let Some(prefix) = id.strip_prefix("tidal:") {
        Ok((MusicSource::Tidal, prefix.to_string()))
    } else if let Some(prefix) = id.strip_prefix("deezer:") {
        Ok((MusicSource::Deezer, prefix.to_string()))
    } else {
        // Default to Qobuz if no prefix
        Ok((MusicSource::Qobuz, id.to_string()))
    }
}