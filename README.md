# Lumio

Lumio is a local-first browser for RAW photography. Point it at folders that already
contain photos and it shows them in place — nothing is imported, moved, renamed, or
modified. It is built with Rust, Tauri 2, Vue 3 and TypeScript.

This is **Milestone 1**: fast, correct folder-based browsing with honest preview-source
labelling. Cataloguing, search, metadata editing and sidecar writing come later.

![Lumio showing a folder of RAW photos in the grid layout](docs/screenshot.jpg)

## What works

- Add one or more library folders with the native folder picker.
- Browse a real nested folder tree, loaded lazily as you expand it.
- Scan the current folder, or include every subfolder recursively.
- View photos in five layouts: grid, masonry, justified, list and filmstrip.
- Double-click any thumbnail to open it in filmstrip view.
- Open any photo in a distraction-free fullscreen viewer from the info panel, the filmstrip
  view, or with the keyboard.
- Associate RAW + JPEG pairs and XMP sidecars that already exist next to the originals.
- See exactly where each preview came from: original image, companion JPEG, camera-embedded
  RAW preview, or a placeholder when none can be extracted.
- Light and dark themes, adjustable panels, thumbnail size, keyboard shortcuts.
- Scan, permission and per-file errors are surfaced without hiding the rest of the folder.

## Non-destructive by design

Milestone 1 is strictly read-only:

- No original file is ever copied, moved, renamed, deleted or written to.
- Thumbnails are written to the application cache directory, never beside your photos.
- XMP sidecars are detected and associated, but never parsed or modified yet.
- Removing a folder from Lumio only forgets it in the app; files on disk are untouched.

## Requirements

- macOS 12 or newer (primary), or a Linux desktop with WebKitGTK (secondary).
- Rust 1.85+ (edition 2024).
- Node.js 20.19+ or 22.12+, and pnpm.

Windows is not a target for Milestone 1.

## Development

```bash
pnpm install
pnpm tauri dev
```

The Vite dev server runs on port 1420 and the Tauri shell loads it automatically.

### Tests and checks

```bash
cargo test -p lumio-core     # Rust domain tests
pnpm test                    # frontend unit tests
pnpm exec vue-tsc --noEmit   # frontend type-check
pnpm build                   # type-check + production frontend build
cargo check -p lumio         # Tauri shell type-check
```

### Production build

```bash
pnpm tauri build
```

The macOS bundle is written to `target/release/bundle/`. The build is unsigned and
unnotarised; it is intended for personal use.

If macOS refuses to create the DMG with a Finder automation error, allow your terminal
to control Finder under System Settings → Privacy & Security → Automation, or build
just the `.app` with `pnpm tauri build --bundles app`.

## How previews work

Lumio deliberately separates the bytes it displays from the original:

1. If a companion JPEG exists next to the RAW file, it is used.
2. Otherwise the largest camera-embedded preview is extracted from the RAW with
   [`rawler`](https://github.com/dnglab/dnglab). No full RAW demosaic happens in M1.
3. If neither exists, the tile shows an honest placeholder instead of a broken image.

The preview source is shown on the tile, in the info panel and in the fullscreen viewer.
Images are resized with a box filter into a bucketed, content-addressed thumbnail cache
under the OS application cache directory.

## Project layout

```
crates/lumio-core/      Domain logic, testable without Tauri
  src/config.rs         Atomic JSON settings and library roots
  src/scan.rs           Folder scanning, RAW/JPEG/XMP association, folder tree
  src/preview.rs        Camera-embedded preview extraction + orientation
  src/thumbnail.rs      Thumbnail cache keys, resizing, encoding
  src/path_utils.rs     Canonicalisation and filesystem metadata helpers
src-tauri/              Thin Tauri shell: state, commands, window setup
src/                    Vue 3 frontend
  components/           Shell, panels, dialogs, viewer
  views/                The five photo layouts
  stores/library.ts     Central application state
  lib/                  Pure layout and formatting helpers (unit-tested)
docs/adr/               Architecture decision records
```

The Rust core owns filesystem behaviour and is independent of Tauri, so it can be tested
headlessly. The Tauri layer only resolves app paths, validates every path sent from the
webview against registered roots, and runs blocking work off the UI thread.

## Milestone 1 scope

In scope: folder roots, nested tree, scanning, association, five views, fullscreen,
preview-source indication, error handling, settings, tests and documentation.

Explicitly deferred: SQLite catalogue, search and filtering, ratings/labels/keywords,
XMP parsing or writing, editing, importing, cloud sync, HEIC/AVIF, map and albums.

## Configuration and data locations

| Data | Location |
| --- | --- |
| Library roots and UI settings | `~/Library/Application Support/com.lumio.desktop/settings.json` |
| Thumbnail cache | `~/Library/Caches/com.lumio.desktop/thumbnails/` |

On Linux these follow the XDG config/cache directories. Deleting the cache is always safe;
Lumio rebuilds previews on demand. A corrupt settings file is preserved with a
`.corrupt-<timestamp>` suffix and Lumio starts with an empty library rather than failing.

## Licence

Lumio is free software licensed under the **GNU General Public License v3.0 or later**
(`GPL-3.0-or-later`). See [LICENSE](LICENSE).
