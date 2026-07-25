//! KuGou Lyrics Provider
//! Ported from RiMusic and APK reference implementations
//! Uses KuGou's lyrics API with base64-encoded content

use bex_core::lyrics::ext::http;
use serde::Deserialize;

const KUGOU_SEARCH_URL: &str = "https://lyrics.kugou.com/search";
const KUGOU_DOWNLOAD_URL: &str = "https://lyrics.kugou.com/download";
const KUGOU_SONG_SEARCH_URL: &str = "https://mobileservice.kugou.com/api/v3/search/song";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    id: i64,
    #[serde(rename = "accesskey")]
    access_key: String,
    #[serde(default)]
    duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DownloadResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct SongSearchResponse {
    data: SongData,
}

#[derive(Debug, Deserialize)]
struct SongData {
    info: Vec<SongInfo>,
}

#[derive(Debug, Deserialize)]
struct SongInfo {
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    duration: Option<i64>,
}

/// Search for lyrics by keyword
pub fn search_by_keyword(keyword: &str) -> Result<Vec<Candidate>, String> {
    let url = format!(
        "{}?ver=1&man=yes&client=mobi&keyword={}",
        KUGOU_SEARCH_URL,
        urlencoding::encode(keyword)
    );

    let resp = http::get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status != 200 {
        return Err(format!("KuGou search failed with status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let result: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(result.candidates)
}

/// Search for lyrics by song hash
pub fn search_by_hash(hash: &str) -> Result<Vec<Candidate>, String> {
    let url = format!(
        "{}?ver=1&man=yes&client=mobi&hash={}",
        KUGOU_SEARCH_URL, hash
    );

    let resp = http::get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status != 200 {
        return Err(format!("KuGou hash search failed with status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let result: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(result.candidates)
}

/// Search for songs to get hash
pub fn search_songs(keyword: &str) -> Result<Vec<SongInfo>, String> {
    let url = format!(
        "{}?version=9108&plat=0&pagesize=8&showtype=0&keyword={}",
        KUGOU_SONG_SEARCH_URL,
        urlencoding::encode(keyword)
    );

    let resp = http::get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status != 200 {
        return Err(format!("KuGou song search failed with status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let result: SongSearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(result.data.info)
}

/// Download lyrics by ID and access key
pub fn download_lyrics(id: i64, access_key: &str) -> Result<String, String> {
    let url = format!(
        "{}?ver=1&man=yes&client=pc&fmt=lrc&id={}&accesskey={}",
        KUGOU_DOWNLOAD_URL, id, access_key
    );

    let resp = http::get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status != 200 {
        return Err(format!("KuGou download failed with status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let result: DownloadResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    
    // Decode base64 content
    let decoded = base64::decode(&result.content)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    String::from_utf8(decoded).map_err(|e| e.to_string())
}

/// Normalize artist and title for KuGou search
pub fn build_keyword(artist: &str, title: &str) -> String {
    let normalized_artist = artist
        .replace(", ", "、")
        .replace(" & ", "、")
        .replace(".", "");
    
    // Remove featuring from title
    let clean_title = if let Some(start) = title.find(" (feat. ") {
        if let Some(end) = title[start..].find(')') {
            format!("{}{}", &title[..start], &title[start + end + 1..])
        } else {
            title.to_string()
        }
    } else {
        title.to_string()
    };
    
    format!("{} - {}", normalized_artist, clean_title)
}

/// Normalize lyrics by removing metadata headers
pub fn normalize_lyrics(raw: &str) -> String {
    let lines: Vec<&str> = raw
        .replace("\r\n", "\n")
        .trim()
        .lines()
        .collect();
    
    let mut to_skip = 0;
    let mut result_lines = Vec::new();
    
    // Skip metadata headers at the beginning
    for (i, line) in lines.iter().enumerate() {
        if to_skip > 0 {
            to_skip -= 1;
            continue;
        }
        
        // Check for metadata lines
        if line.starts_with("[ti:") || 
           line.starts_with("[ar:") || 
           line.starts_with("[al:") || 
           line.starts_with("[by:") ||
           line.starts_with("[hash:") ||
           line.starts_with("[sign:") ||
           line.starts_with("[qq:") ||
           line.starts_with("[total:") ||
           line.starts_with("[offset:") ||
           line.starts_with("[id:") ||
           line.contains("]Written by：") ||
           line.contains("]Lyrics by：") ||
           line.contains("]Composed by：") ||
           line.contains("]Producer：") ||
           line.contains("]作曲 : ") ||
           line.contains("]作词 : ") {
            to_skip = 0;
            continue;
        }
        
        result_lines.push(*line);
    }
    
    result_lines.join("\n")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}
