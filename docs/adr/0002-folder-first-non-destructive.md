# ADR 0002 — Folder-first, strictly non-destructive browsing

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Photographers already have a folder structure they trust. Importing files into a managed
library duplicates data, breaks existing workflows and risks data loss. Lumio's premise
is that the existing filesystem is the source of truth.

## Decision

Milestone 1 never writes to, moves, renames or deletes an original file.

- Folders are registered as **roots**; the user picks them with the native dialog.
- Scanning only reads metadata and bytes.
- Generated thumbnails go to the OS application cache directory.
- XMP sidecars, when present, are associated and displayed but not parsed or edited.
- Removing a root only removes it from Lumio's settings.

Hidden entries, macOS packages (for example `Photos Library.photoslibrary`), and
symlinked directories are skipped. This avoids traversing enormous opaque bundles or
loops.

## Consequences

- There is no import step and no migration of the user's data.
- The library can be rebuilt from the filesystem at any time.
- Editing, ratings and keywords must later be designed around XMP sidecars or a
  rebuildable catalogue rather than hidden application copies.
