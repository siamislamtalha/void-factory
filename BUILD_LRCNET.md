# Build Instructions for Updated lrcnet Plugin

## Quick Build (Recommended)

The plugin uses the existing GitHub Actions workflow. To build:

### Option 1: GitHub Actions (Easiest)
1. Push your changes to the `plugins` repository
2. The `.github/workflows/bex-factory.yml` will automatically build all plugins
3. Download the `.bex` file from the GitHub release
4. Install it in your app

### Option 2: Manual Build with bex CLI
If you have the `bex` CLI tool installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins
bex pack lrcnet
```

This will create `lrcnet.bex` in the current directory.

### Option 3: Manual Build with cargo-component
If you have Rust and cargo-component installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins\lrcnet
cargo component build --release
```

The compiled `.wasm` file will be in `target/wasm32-unknown-unknown/release/`.

## What Changed

The lrcnet plugin now fetches lyrics from 3 sources with automatic fallback:

1. **LRCLIB** (primary) - https://lrclib.net/api
2. **KuGou** (fallback) - https://lyrics.kugou.com
3. **BetterLyrics** (fallback) - https://lyrics-api.boidu.dev

## App Compatibility

✅ **No app code changes needed** - The plugin interface is identical:
- Same plugin ID: `lyrics-provider.bloomfactory.lrcnet`
- Same function signatures: `get_lyrics()`, `search()`, `get_lyrics_by_id()`
- Same data structures returned
- Publisher: Void Music

## Installation

Once you have the `.bex` file:

1. Open your app
2. Go to Plugin Manager
3. Install the new `lrcnet.bex` file
4. The app will automatically use the updated plugin

## Verification

After installation:
1. Play any song
2. Open lyrics view
3. Check that lyrics load successfully
4. The app will automatically try all 3 sources if needed

## Troubleshooting

If the plugin fails to load:
- Ensure the `.bex` file was built successfully
- Check that the plugin ID matches: `lyrics-provider.bloomfactory.lrcnet`
- Verify the manifest version is compatible with your app
- Confirm publisher is set to: Void Music

## API Notes

All APIs used are free and require no authentication:
- LRCLIB: Open source lyrics database
- KuGou: Chinese lyrics service (no auth required)
- BetterLyrics: Community lyrics API (no auth required)
