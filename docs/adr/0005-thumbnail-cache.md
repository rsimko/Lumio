# ADR 0005 — Bucketed thumbnail cache served over the asset protocol

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Decoding a camera preview and downscaling it takes tens to hundreds of milliseconds.
A grid can request hundreds of tiles, often the same file at several sizes (grid,
filmstrip, info panel, fullscreen). Re-encoding on every render is not viable, and the
webview must not be given arbitrary filesystem access.

## Decision

- Generate JPEG thumbnails into size buckets: 256, 512, 1024, 2048 and 2560 px on the
  long edge. Requests round up to the next bucket.
- Name cache entries by a hash of the source path plus size and modification time, so
  changed files are naturally re-rendered and stale entries become unreachable.
- Store `CACHE_VERSION` in the path; bumping it invalidates old caches without migration.
- Serve files with Tauri's asset protocol, scoped in `tauri.conf.json` to
  `$APPCACHE/**` only. The webview never receives a general filesystem scope.
- Re-validate every path sent from the webview against the registered roots before
  reading it, then refresh size/mtime from disk.
- Coalesce concurrent requests for the same asset and edge, and limit concurrent
  thumbnail jobs with a semaphore of 4.
- The frontend deduplicates requests and requests a bucket based on CSS size × device
  pixel ratio (capped at 3×).

## Consequences

- Scrolling back through a folder is effectively instant.
- The cache is disposable and rebuildable; clearing it never touches originals.
- Editing a file in place produces a new key and a fresh thumbnail.
- Multiple sizes of the same photo are cached independently, trading a little disk space
  for predictable latency.
