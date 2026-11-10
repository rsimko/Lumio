# ADR 0004 — Camera-embedded previews for RAW browsing

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Raw files are 20–100 MB and need demosaicing, white balance and tone mapping before they
look like a photograph. Rendering full RAW data for every grid tile is far too slow for a
responsive browser, and M1 explicitly excludes editing.

Almost every camera writes one or more developed JPEG previews inside the RAW container.
A companion JPEG shot alongside the RAW is even better: it is the photographer's intended
rendering at full resolution.

## Decision

Preview selection order:

1. **Companion JPEG** with the same stem in the same directory.
2. **Largest camera-embedded preview** extracted with the pure-Rust `rawler` crate
   (`preview_image`, falling back to `thumbnail_image`), with EXIF orientation applied.
3. **Placeholder** if neither can be produced.

The source is always surfaced in the UI (`original`, `companionJpeg`, `embeddedRaw`,
`placeholder`) so the user is never misled about what they are looking at.

Full RAW demosaicing is not part of M1. `rawler` is used instead of shelling out to
`dcraw`/`libraw` for a self-contained, cross-platform, memory-safe build. A benchmark
across 91 real RAW files (CR2, CR3, NEF, ARW, DNG, RAF and others) extracted a preview
from every file.

Resizing uses `image::imageops::thumbnail` (box filter). On a 45 MP preview this is about
8× faster than Lanczos3 and avoids ringing artefacts; thumbnail generation averages
around 80 ms per file from cold cache.

## Consequences

- Grid browsing is fast and never blocks on demosaicing.
- Displayed colour reflects the camera/JPEG rendering, not an editable RAW pipeline.
- The preview source must remain visible in the UI as a trust feature.
- A future editing milestone will need a separate full-resolution render path.
