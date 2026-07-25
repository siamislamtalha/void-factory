//! BetterLyrics Provider
//! Ported from Metrolist BetterLyrics implementation
//! Uses lyrics-api.boidu.dev for TTML format lyrics

use bex_core::lyrics::ext::http;
use serde::Deserialize;

const BETTERLYRICS_BASE_URL: &str = "https://lyrics-api.boidu.dev";

#[derive(Debug, Deserialize)]
struct BetterLyricsResponse {
    #[serde(default)]
    ttml: Option<String>,
}

/// Fetch TTML lyrics from BetterLyrics API
pub fn fetch_lyrics(
    artist: &str,
    title: &str,
    duration: Option<i64>,
    album: Option<&str>,
) -> Result<String, String> {
    let mut params = vec![
        ("s".to_string(), title.to_string()),
        ("a".to_string(), artist.to_string()),
    ];
    
    if let Some(d) = duration {
        if d > 0 {
            params.push(("d".to_string(), d.to_string()));
        }
    }
    
    if let Some(alb) = album {
        if !alb.trim().is_empty() {
            params.push(("al".to_string(), alb.trim().to_string()));
        }
    }
    
    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    
    let url = format!("{}/getLyrics?{}", BETTERLYRICS_BASE_URL, query_string);

    let resp = http::get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Accept", "application/json")
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status != 200 {
        return Err(format!("BetterLyrics API failed with status {}", resp.status));
    }

    let body = String::from_utf8(resp.body).map_err(|e| e.to_string())?;
    let result: BetterLyricsResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    
    match result.ttml {
        Some(ttml) if !ttml.trim().is_empty() => Ok(ttml),
        _ => Err("No TTML content found".to_string()),
    }
}

/// Convert TTML to LRC format
pub fn ttml_to_lrc(ttml: &str) -> Result<String, String> {
    let mut lrc_lines = Vec::new();
    
    // Simple TTML to LRC conversion
    // TTML format: <p begin="1.23s">Lyrics text</p>
    // LRC format: [00:01.23]Lyrics text
    
    for line in ttml.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with("<p") {
            continue;
        }
        
        // Extract begin time
        if let Some(start) = line.find("begin=\"") {
            let time_part = &line[start + 7..];
            if let Some(end) = time_part.find("\"") {
                let time_str = &time_part[..end];
                if let Some(seconds) = parse_ttml_time(time_str) {
                    // Extract text content
                    if let Some(text_start) = line.find('>') {
                        if let Some(text_end) = line.rfind('<') {
                            let text = &line[text_start + 1..text_end];
                            if !text.trim().is_empty() {
                                let lrc_time = format_lrc_time(seconds);
                                lrc_lines.push(format!("[{}]{}", lrc_time, text.trim()));
                            }
                        }
                    }
                }
            }
        }
    }
    
    if lrc_lines.is_empty() {
        Err("Failed to convert TTML to LRC".to_string())
    } else {
        Ok(lrc_lines.join("\n"))
    }
}

/// Parse TTML time format (e.g., "1.23s" or "45.678s")
fn parse_ttml_time(time_str: &str) -> Option<f64> {
    let time_str = time_str.trim_end_matches('s');
    time_str.parse::<f64>().ok()
}

/// Format seconds to LRC time format [MM:SS.ms]
fn format_lrc_time(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let millis = ((seconds - total_secs) * 100.0) as u64;
    
    format!("{:02}:{:02}.{:02}", mins, secs, millis)
}
