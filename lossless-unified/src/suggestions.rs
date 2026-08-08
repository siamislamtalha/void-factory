use crate::types::*;
use crate::mapper::map_media_item;
use bex_core::resolver::discovery::Section;
use bex_core::resolver::types::MediaItem;

/// Fetch home sections with recommendations from all services
pub async fn fetch_home_sections() -> Result<Vec<Section>, String> {
    // Initialize clients
    crate::client::ensure_clients_initialized().await;
    
    let mut sections = Vec::new();
    
    // Use shared clients from client.rs
    use crate::client::{QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
    
    // Qobuz: New Releases
    {
        let client = QOBUZ_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match client.get_featured("new-releases", 20).await {
                Ok(albums) => {
                    sections.push(Section {
                        id: "qobuz_new_releases".to_string(),
                        title: "Qobuz: New Releases".to_string(),
                        items: albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect(),
                        page_token: None,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to fetch Qobuz new releases: {}", e);
                }
            }
        }
    }
    
    // Tidal: Featured
    {
        let client = TIDAL_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match client.get_featured(20).await {
                Ok(albums) => {
                    sections.push(Section {
                        id: "tidal_featured".to_string(),
                        title: "Tidal: Featured Albums".to_string(),
                        items: albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect(),
                        page_token: None,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to fetch Tidal featured: {}", e);
                }
            }
        }
    }
    
    // Deezer: Charts
    {
        let client = DEEZER_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match client.get_featured(20).await {
                Ok(albums) => {
                    sections.push(Section {
                        id: "deezer_charts".to_string(),
                        title: "Deezer: Charts".to_string(),
                        items: albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect(),
                        page_token: None,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to fetch Deezer charts: {}", e);
                }
            }
        }
    }
    
    // SoundCloud: Trending (using search for now)
    {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match client.search_tracks("trending", 20).await {
                Ok(tracks) => {
                    sections.push(Section {
                        id: "soundcloud_trending".to_string(),
                        title: "SoundCloud: Trending".to_string(),
                        items: tracks.into_iter().map(|t| MediaItem::Track(crate::mapper::map_track(t))).collect(),
                        page_token: None,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to fetch SoundCloud trending: {}", e);
                }
            }
        }
    }
    
    // Add a combined "Best Quality" section
    sections.push(Section {
        id: "best_quality".to_string(),
        title: "Highest Quality (24-bit FLAC)".to_string(),
        items: vec![], // Will be populated dynamically
        page_token: None,
    });
    
    if sections.is_empty() {
        return Err("Failed to fetch any home sections. At least one service should be available.".to_string());
    }
    
    Ok(sections)
}

/// Load more items for a specific section
pub async fn load_more_section(section_id: &str, page_token: &str) -> Result<Vec<MediaItem>, String> {
    // Initialize clients
    crate::client::ensure_clients_initialized().await;
    
    let limit = 50;
    
    // Use shared clients from client.rs
    use crate::client::{QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
    
    match section_id {
        "qobuz_new_releases" => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_featured("new-releases", limit).await
                    .map(|albums| albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect())
            } else {
                Err("Qobuz client not available".to_string())
            }
        }
        "tidal_featured" => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_featured(limit).await
                    .map(|albums| albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect())
            } else {
                Err("Tidal client not available".to_string())
            }
        }
        "deezer_charts" => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_featured(limit).await
                    .map(|albums| albums.into_iter().map(|a| MediaItem::Album(crate::mapper::map_album(a))).collect())
            } else {
                Err("Deezer client not available".to_string())
            }
        }
        "soundcloud_trending" => {
            let client = SOUNDCLOUD_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks("trending", limit).await
                    .map(|tracks| tracks.into_iter().map(|t| MediaItem::Track(crate::mapper::map_track(t))).collect())
            } else {
                Err("SoundCloud client not available".to_string())
            }
        }
        _ => Err(format!("Unknown section: {}", section_id)),
    }
}