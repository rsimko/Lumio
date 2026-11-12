# Lumio

IMPORTANT !!!
This is a persolan project and I am building it for my personal needs and it is adjusted to my workflow. I am not planning to make an official release version. Application is vibe-coded and I do not take any responsibility if it doesn't work as you expect, That said, feel free to fork it and do whatever you please with the codebase.

Lumio is a local-first browser for RAW photography. Point it at folders that already
contain photos and it shows them in place — nothing is imported, moved or renamed. Photos are
only written when you explicitly edit metadata, and even then only the XMP properties Lumio
owns are changed. It is built with Rust, Tauri 2, Vue 3 and TypeScript.

This is **Milestone 1**: fast, correct folder-based browsing with honest preview-source
labelling, a copy-only camera-card ingest, and in-place ratings, labels, culling, favorites,
IPTC and location metadata.
Cataloguing, search and hierarchical keyword editing come later.

![Lumio showing a folder of RAW photos in the grid layout](docs/screenshot.jpg)

## What works

- Add library folders with the native folder picker, and remove them again.
- Right-click any folder to create a subfolder inside it or open the same removal choices.
- Refresh the selected folder from the library header, or refresh a single folder from its
  right-click menu without rescanning the rest of the library.
- Every folder row offers the same two actions: remove it from the library (files stay on disk) or
  move it to the system Trash. Hidden subfolders can be restored by adding them again.
- Browse a real nested folder tree, loaded lazily as you expand it.
- Scan the current folder, or include every subfolder recursively.
- View photos in five layouts: grid, masonry, justified, list and filmstrip. The bottom
  navigation strip belongs to the filmstrip view; grid, masonry, justified and list stay
  strip-free. Drag the strip's top edge to make its thumbnails larger or smaller.
- Select one or many photos: click, ⌘/Ctrl-click to toggle, Shift-click for a range,
  ⌘/Ctrl+A to select all, Shift+arrows to extend and Escape to clear.
- Drag any selected photo — or a single unselected photo — onto a folder row to move the
  whole group, including its companion JPEG, XMP sidecar and extra associations. Moves work
  between registered library roots and refuse to overwrite existing files.
- Ingest a camera card or folder from the toolbar: choose source folders or individual files,
  a destination and a folder/filename template, then review a Dry Run before copying.
  RAW, JPEG and XMP files are grouped into one photo entity and copied together.
- Ingest templates support `{YYYY}`, `{MM}`, `{DD}`, `{hh}`, `{mm}`, `{ss}`, `{job_code}`,
  `{custom_text}`, `{default_filename}` and `{counter}`. `/` creates folder levels in folder
  templates, and the counter starts and zero-padding are configurable. Camera make/model
  tokens are deliberately excluded.
- Save, update and delete named templates. Ingest settings persist between sessions.
- Ingest detects existing files, duplicate output names and partial groups. Conflicts default
  to Skip, with per-photo Keep both (auto-rename) and explicitly confirmed Replace. Nothing is
  ever silently overwritten.
- Ingest copies into a hidden staging folder, verifies every copy with BLAKE3 and commits with
  atomic renames; a failed group rolls back, and a replaced file is restored on failure.
- Optionally write IPTC title, description, creator, copyright and keywords while ingesting.
  IPTC fields are literal text, not templates. RAW photos get an XMP sidecar, standalone JPEGs
  get embedded XMP/IPTC, and every unrelated XMP field is preserved. ExifTool is used for writing
  and the result is read back to verify.
- Ingest can add the destination folder to the library at the end, or leave it unregistered.
- Delete photos from the library only or from disk, from the toolbar, the info panel, the
  Delete key, or by right-clicking a photo. Disk deletion moves every associated file to the
  system Trash after an explicit warning; library-only deletion hides the group and can be
  reversed in Settings → Library → Hidden photos.
- Double-click any thumbnail to open it in filmstrip view.
- See photo counts beside every folder in the tree; counts follow the Include subfolders setting.
- Companion JPEG and XMP associations are shown as badges in list view and the info panel.
  Toggle **Assoc.** in the list header to reveal every associated JPG and XMP file as indented,
  connected rows beneath the RAW they belong to.
- Open any photo in a distraction-free fullscreen viewer from the top toolbar, the filmstrip
  view, or with the keyboard.
- Rate photos from 1–5 stars, apply one of five standard color labels, mark Pick/Reject/Unflag
  and toggle favorites from the info panel, with `1`–`5`, `Option`+`6`–`0`, `P`, `X`, `U` and
  `Shift+F` working in both browse and fullscreen. Multi-selection shows mixed values and applies
  changes as one batch; tiles, list rows and the viewer show compact indicators.
- Edit IPTC title, headline, description, creator, credit, copyright and source from a dedicated
  info-panel section. Edits save automatically on blur or Enter, queue in order, and apply to
  every selected photo. Existing `dc:title`, `dc:description` and `dc:rights` language variants
  are preserved when the `x-default` value changes.
- Edit city, state/province, country, country code and sublocation, plus latitude, longitude and
  altitude. Coordinates accept decimal degrees or simple DMS, are validated before writing, and
  can be cleared independently. A compact offline map placeholder shows the location pin and
  offers **Open in Maps**. Camera EXIF GPS is preferred when present; switch between the Camera
  and XMP sources, or edit a coordinate to store an XMP position. Camera GPS can also be copied
  into XMP with one click.
- Edits update the existing XMP rather than regenerating it. RAW, TIFF, PNG and WebP use a
  sidecar; a standalone JPEG embeds XMP; a JPEG with a sidecar writes only that sidecar. Writes
  are staged, read back field-by-field, and checked so unrelated XMP (Darktable history, masks,
  private namespaces and unknown tags) survives. A revision fingerprint rejects stale edits.
- Flat keywords are read from `dc:subject` and edited as chips in the info panel one photo at a
  time. Hierarchical keywords are read from `lr:hierarchicalSubject` and shown read-only.
- Associate RAW + JPEG pairs and XMP sidecars that already exist next to the originals.
- See exactly where each preview came from: original image, companion JPEG, camera-embedded
  RAW preview, or a placeholder when none can be extracted. Preview badges use the short
  `EMBEDDED`, `JPG`, `RAW` or file-type labels.
- Switch the info panel tile between the preview and an RGB/luminance histogram computed from
  the same cached preview; both tabs share one tile and the histogram opens by default. RAW
  photos have a RAW button that recomputes the histogram from decoded sensor samples.
- Collapse info-panel sections such as Metadata, Keywords, IPTC, Location, Camera and lens, File
  and Associations to keep the panel compact. Camera make/model, lens, shutter, aperture, ISO,
  focal length and GPS coordinates are read from embedded EXIF; RAW files also try the rawler
  decoder and their companion JPEG. Reading this information does not require ExifTool.
- Light and dark themes, a unified teal accent palette, adjustable panels, thumbnail size,
  corner roundness and keyboard shortcuts.
- A tabbed Settings dialog keeps appearance, library, editor, ingest and storage preferences
  separate, with every section composed from independent cards.
- Settings for any installed interface or monospace font family, with Avenir and bundled
  JetBrains Mono as the defaults, plus text size.
- Scan, permission and per-file errors are surfaced without hiding the rest of the folder.

## Non-destructive by default

Browsing is strictly read-only. The only writes that ever touch a photo or sidecar happen when
you explicitly change metadata, and they are limited to the XMP target and fields described
above:

- No original file is written just by browsing or generating previews. A metadata edit touches
  only the effective target: a sidecar when one exists, or embedded XMP in a standalone JPEG.
- Thumbnails are written to the application cache directory, never beside your photos.
- XMP sidecars are parsed for capture dates and associations. Ingest writes IPTC values only to
  destination copies; source sidecars and originals are never modified.
- Removing a folder from Lumio only hides it in the app; files on disk are untouched. Hidden
  subfolders can be restored with Add Folder.
- Dragging photos between folders and deleting photos are explicit actions. Both expand the
  photo into its primary file, companion JPEG, XMP sidecar and any extra associated files.
  Moves refuse to overwrite an existing destination, and a failed batch is rolled back where
  possible. Cross-volume moves fall back to copy-then-remove.
- Delete from library only hides the photo group in Lumio. Every file stays on disk and the
  group can be restored from Settings → Library → Hidden photos.
- The optional Delete from disk action moves a photo group, root or subfolder to the system
  Trash, where it can still be restored. It requires acknowledging a warning, and Lumio refuses
  filesystem roots, home folders, system folders, mount points, hidden/package folders and paths
  that overlap another registered library folder.
- New folders are created directly inside the right-clicked folder. Names that are empty, hidden,
  contain path separators, colons or control characters, or collide with an existing folder are
  rejected.
- Ingest is copy-only. Sources are never renamed, moved or written to, and the operation refuses
  unsafe destinations such as a folder inside a source.

## Metadata mapping

| Lumio field | XMP property | Notes |
| --- | --- | --- |
| Rating | `xmp:Rating` | 0–5 stars. Adobe's `-1` is read as zero stars plus Reject. |
| Color label | `xmp:Label` | Red, Yellow, Green, Blue or Purple. |
| Cull flag | `lumio:Cull` | `pick` or `reject`, independent of the star rating. |
| Favorite | `lumio:Favorite` | Boolean. |
| Keywords | `dc:subject` | Flat keyword bag. |
| Hierarchical keywords | `lr:hierarchicalSubject` | `Parent\|Child` paths. |
| Title | `dc:title` | RDF language alternative; edits write `x-default`. |
| Description | `dc:description` | RDF language alternative; edits write `x-default`. |
| Creator | `dc:creator` | The first creator is shown; editing replaces the creator list. |
| Copyright | `dc:rights` | RDF language alternative; edits write `x-default`. |
| Headline | `photoshop:Headline` | Plain text. |
| Credit | `photoshop:Credit` | Plain text. |
| Source | `photoshop:Source` | Plain text. |
| City | `photoshop:City` | Plain text. |
| State / Province | `photoshop:State` | Plain text. |
| Country | `photoshop:Country` | Plain text. |
| Location | `Iptc4xmpCore:Location` | Sublocation or landmark. |
| Country code | `Iptc4xmpCore:CountryCode` | Normalised to uppercase. |
| Latitude | `exif:GPSLatitude` | Signed decimal degrees, -90 to 90. |
| Longitude | `exif:GPSLongitude` | Signed decimal degrees, -180 to 180. |
| Altitude | `exif:GPSAltitude` + `exif:GPSAltitudeRef` | Metres relative to sea level. |

The private namespace is `https://lumio.app/ns/1.0/`. Lumio generates an ExifTool tag
configuration at `exiftool-lumio.config` in the application cache on startup. Unknown labels
or out-of-range ratings are surfaced as warnings but do not block edits to other fields;
unreadable or malformed XMP does block the write.

## Requirements

- macOS 12 or newer (primary), or a Linux desktop with WebKitGTK (secondary).
- Rust 1.85+ (edition 2024).
- Node.js 20.19+ or 22.12+, and pnpm.
- ExifTool is optional for browsing and ingest scanning (Lumio falls back to `rawler`,
  `kamadak-exif` and its native XMP parser) but is required to write IPTC and location metadata
  and to edit photo ratings, labels, culling, favorites or keywords. Set its path in
  Settings → Ingest if it is not on `PATH`.

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

The preview source is shown as a short label on tiles, a tooltip in the filmstrip and a badge
in the fullscreen viewer. Images are resized with a box filter into a bucketed,
content-addressed thumbnail cache under the OS application cache directory.

## Project layout

```
crates/lumio-core/      Domain logic, testable without Tauri
  src/config.rs         Atomic JSON settings and library roots
  src/assets.rs         Asset file groups, safe moves, Trash and hidden groups
  src/scan.rs           Folder scanning, RAW/JPEG/XMP association, folder tree
  src/metadata.rs       XMP read/write, preservation checks, JPEG segment handling
  src/ingest/           Source scanning, templates, planning, verified copy, IPTC
  src/preview.rs        Camera-embedded preview extraction + orientation
  src/thumbnail.rs      Thumbnail cache keys, resizing, encoding
  src/path_utils.rs     Canonicalisation and filesystem metadata helpers
src-tauri/              Thin Tauri shell: state, commands, window setup
src/                    Vue 3 frontend
  components/           Shell, panels, dialogs, viewer
  views/                The five photo layouts
  stores/library.ts     Central application state
  stores/ingest.ts      Ingest dialog, plan, progress and template state
  lib/                  Pure layout, selection and formatting helpers (unit-tested)
docs/adr/               Architecture decision records
```

The Rust core owns filesystem behaviour and is independent of Tauri, so it can be tested
headlessly. The Tauri layer only resolves app paths, validates every path sent from the
webview against registered roots, and runs blocking work off the UI thread.

## Milestone 1 scope

In scope: folder roots, nested tree, scanning, association, five views, fullscreen,
preview-source indication, copy-only ingest with templates and IPTC writing, in-place ratings,
color labels, culling and favorites, editable IPTC/location/geolocation metadata, flat keyword
editing, a location-map placeholder, error handling, settings, tests and documentation.

Explicitly deferred: SQLite catalogue, search and filtering, hierarchical keyword editing,
cloud sync, HEIC/AVIF, a full interactive map and albums.

## Configuration and data locations

| Data | Location |
| --- | --- |
| Library roots, UI settings and ingest templates | `~/Library/Application Support/com.lumio.desktop/settings.json` |
| Thumbnail cache | `~/Library/Caches/com.lumio.desktop/thumbnails/` |

On Linux these follow the XDG config/cache directories. Deleting the cache is always safe;
Lumio rebuilds previews on demand. A corrupt settings file is preserved with a
`.corrupt-<timestamp>` suffix and Lumio starts with an empty library rather than failing.

## Licence

Lumio is free software licensed under the **GNU General Public License v3.0 or later**
(`GPL-3.0-or-later`). See [LICENSE](LICENSE).
