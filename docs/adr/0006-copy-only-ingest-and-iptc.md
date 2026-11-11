# ADR 0006 — Copy-only ingest with verified commits and destination-only IPTC

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

Lumio is folder-first and non-destructive. Users still need to move a camera card into a
structured photo library: one logical photo is a group of files (RAW, companion JPEG, one or
more XMP sidecars), and those files must never be split or overwritten by accident.

Template-based renaming is easy to get wrong in two places: the Dry Run can disagree with the
real run, and a partially completed group can leave the destination in a mixed state. Metadata
writing adds a third risk: an XMP rewrite can silently drop fields that Lumio does not manage.

## Decision

- **One entity per `(directory, lowercase stem)`.** All associated RAW, JPEG and XMP files
  belong to one entity. A standalone JPEG is an entity only when no RAW shares its stem.
  Multiple RAWs with the same stem stay in one entity.
- **Dry Run and ingest share one plan.** Planning scans sources, reads metadata, associates
  files, renders templates, assigns counters and detects conflicts once. The serialized plan is
  stored in Tauri state; `run_ingest` consumes that exact plan and never rescans or re-renders a
  template. Conflict-resolution changes rebuild paths only from the stored base names.
- **Capture metadata precedence** is RAW ExifTool, RAW built-in, JPEG, XMP, then file
  modification time with a visible warning. ExifTool is used when available; otherwise
  `rawler` and `kamadak-exif` keep ingest working without external tools.
- **Copy-only, staged and verified.** Each entity is copied into
  `<destination>/.lumio-ingest-tmp/<planId>/`, verified with BLAKE3 and then committed with
  atomic renames. Existing files are moved aside first when Replace was explicitly confirmed.
  Any failure rolls the whole entity back, including restoring replaced files. Temporary folders
  are removed afterwards.
- **Conflicts are explicit.** Existing files and duplicate output names make an entity a
  conflict. The default is Skip. Keep both renames the whole entity with `_2`, `_3`, … and never
  lands on another existing file. Replace requires a per-entity confirmation in the UI.
- **IPTC writing targets destination copies only.** IPTC fields are literal text, not templates.
  RAW entities receive `XMP-dc:Title`, `Description`, `Creator`, `Rights` and `Subject` in their
  copied XMP sidecar (`XMP-dc:*)`; standalone JPEGs receive the same XMP properties plus legacy
  IPTC IIM. Only non-empty requested fields are passed to ExifTool, so unrelated XMP fields,
  namespaces and the rest of the packet are preserved. A minimal valid packet is created when a
  RAW entity has no source sidecar. ExifTool is mandatory for IPTC and every write is read back
  for verification. Source files and source sidecars are never modified.
- **XMP sidecar naming** is `{filename}.{ext}.xmp` by default (for example `NEW.CR3.xmp`), with a
  persisted setting for `{filename}.xmp` (`NEW.xmp`).
- **Library registration** reuses the folder-addition path. Adding a destination already covered
  by a root succeeds as a no-op with `alreadyInLibrary`, so ingest cannot create duplicate roots.

## Consequences

- Ingest doubles disk usage for the copied files while staging; verification happens before the
  commit, so the visible destination only ever contains complete groups.
- The stored plan is a snapshot. If templates, job code, counter settings or the requested
  ExifTool setting change after a Dry Run, the UI marks the plan stale and requires a new Dry Run
  instead of executing outdated paths. The plan stores the requested ExifTool setting separately
  from the resolved executable, so auto-detection does not look like a settings change.
- IPTC writes go through ExifTool, which reformats the XMP packet. Fields are preserved
  semantically, not byte-for-byte.
- Replacing a file is not atomic across the group in the filesystem sense; it is atomic from the
  user's perspective because failures restore backups and remove committed files.
