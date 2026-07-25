# Universal Downloader Plugin

## Overview

Universal music downloader plugin with multi-method fallback support for maximum reliability. This plugin implements multiple download methods extracted from various APK reference implementations with automatic failover when one method fails.

## Features

### YouTube Download Methods
- **Innertube API** with 8 backup API keys from various music apps
- **Multiple client fallbacks**: ANDROID_VR, IOS, TVHTML5, WEB_REMIX
- **Signature cipher decoding** for encrypted streams
- **Range request support** to avoid YouTube throttling
- **Visitor data extraction and caching**

### JioSaavn Download Methods
- **DES-ECB decryption** for encrypted stream URLs
- **Multiple server endpoints** with automatic rotation
- **Quality selection** (96kbps, 128kbps, 160kbps, 320kbps)
- **Remote config support** for dynamic server updates

### Direct HTTP Methods
- **Fallback to direct HTTP streaming** for unsupported sources
- **Support for various audio formats** (m4a, mp3, webm)
- **Progressive download** with resume capability

## Credential Pools

### YouTube API Keys (8 keys total)
The plugin uses a credential rotator that automatically cycles through 8 API keys from various music apps:
- `AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30` (WEB_REMIX - InnerTune, Kreate)
- `AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX3` (WEB - InnerTune, OuterTune, OpenTune)
- `AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw` (PoToken - RiMusic, Kreate, OuterTune)
- `AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI` (ANDROID_MUSIC - Musify, InnerTune, Kreate)
- `AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w` (ANDROID - InnerTune, Kreate)
- `AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc` (IOS - Musify, Kreate)
- `AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8` (TVHTML5 - InnerTune, Kreate)
- `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8` (Additional - Musify)

**Credential Rotation**: If one key fails, the plugin automatically rotates to the next available key.

### JioSaavn Credentials
- **DES-ECB decryption key**: `38346591`
- **Default servers with automatic rotation**:
  - `saavn.echomusic.fun`
  - `saavn1.echomusic.fun`
  - `saavn2.echomusic.fun`
  - `www.jiosaavn.com`

**Server Rotation**: JioSaavn uses a server rotator that automatically cycles through available servers on failure.

## Installation

### Option 1: GitHub Actions (Easiest)
1. Push your changes to the `plugins` repository
2. The `.github/workflows/bex-factory.yml` will automatically build all plugins
3. Download the `.bex` file from the GitHub release
4. Install it in your app

### Option 2: Manual Build with bex CLI
If you have the `bex` CLI tool installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins
bex pack universal-downloader
```

This will create `universal-downloader.bex` in the current directory.

### Option 3: Manual Build with cargo-component (Required for Windows)
**Note**: Standard `cargo build` will fail on Windows due to missing MSVC linker. Use cargo-component instead.

If you have Rust and cargo-component installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins\universal-downloader
cargo component build --release
```

The compiled `.wasm` file will be in `target/wasm32-unknown-unknown/release/`.

To install cargo-component:
```bash
cargo install cargo-component --locked
```

## Usage

After installation:
1. Use the download function in your app
2. The plugin will automatically try multiple download methods
3. If one method fails, it automatically falls back to the next method
4. Results are returned with the best available stream

## App Compatibility

✅ **No app code changes needed** - The plugin uses the standard content-resolver interface:
- Plugin ID: `content-resolver.bloomfactory.universal-downloader`
- Standard function signatures: `get_streams()`, etc.
- Standard data structures returned
- Publisher: Void Music

## Architecture

The plugin implements a priority-based fallback system:

1. **YouTube Innertube API** (highest priority)
   - Tries ANDROID_VR client first
   - Falls back to IOS client
   - Falls back to TVHTML5 client
   - Falls back to WEB_REMIX client
   - Rotates API keys on failure

2. **JioSaavn DES-ECB** (medium priority)
   - Tries primary server first
   - Rotates to backup servers on failure
   - Applies DES-ECB decryption to encrypted URLs

3. **Direct HTTP** (fallback priority)
   - Used when all other methods fail
   - Supports direct URL downloads
   - Progressive download with resume capability

## Troubleshooting

If the plugin fails to load:
- Ensure the `.bex` file was built successfully
- Check that the plugin ID matches: `content-resolver.bloomfactory.universal-downloader`
- Verify the manifest version is compatible with your app
- Confirm publisher is set to: Void Music

## Implementation Details

This plugin contains the complete implementations:
- **credentials.rs** - Credential pool with 8 YouTube API keys and JioSaavn server rotator
- **youtube_downloader.rs** - Full YouTube client with multi-client fallback and credential rotation
- **jiosaavn_downloader.rs** - Full JioSaavn client with DES-ECB decryption and server rotation
- **http_downloader.rs** - Direct HTTP fallback with resume capability
- **stream_resolver.rs** - Stream resolver with automatic fallback coordination

## Performance

- **Automatic credential rotation** prevents rate limiting
- **Multi-client fallback** ensures high availability
- **Server rotation** for JioSaavn prevents single-point failures
- **Progressive download** with resume capability for large files
- **Optimized WASM build** for fast loading
