# ADR 0004 — Internal photo moves and hidden-asset groups

- **Status:** Accepted
- **Date:** 2026-11-10

## Context

Milestone 1 has no catalogue database. Photos are scanned directly from disk, and the only
persisted library state is the root list, hidden folders and UI preferences. Users now need
to move photos between folders by dragging them in Lumio, and to delete one or many photos.

Photo deletion has two meanings in a folder-first app:

1. **Delete from library** — stop showing the photo in Lumio, but leave every file intact.
2. **Delete from disk** — move the photo and its associated files out of the library.

Every photo is a group of files: the primary RAW/image, a companion JPEG, an XMP sidecar and
potentially extra sidecars or companions. Operating on only the primary would strand files.

## Decision

- Internal drag-and-drop uses HTML5 drag events with a private MIME type. Native Finder drops
  remain disabled (`dragDropEnabled: false`) and are deferred.
- A move is preflighted as one batch: every file is canonicalised, checked to be a regular file
  under a registered root, and mapped to a destination name. Existing targets and duplicate
  names inside the batch abort before any file moves. Moves may cross registered roots.
- Files are moved with `rename`. If the source and destination are on different volumes, Lumio
  copies and then removes the source. A later failure rolls already-moved files back where
  possible.
- Delete from disk expands and deduplicates the same file groups and moves each file to the
  operating-system Trash with the same `NSFileManager` path used for folder removal. This stays
  recoverable and avoids Finder automation prompts.
- Because there is no catalogue, Delete from library persists an `excluded_assets` list in
  `settings.json`. Each entry records the stable primary-path hash, the display name, the time
  hidden and every file path in the group. Scans, counts and listings filter these paths so a
  hidden photo and its associated files do not reappear.
- Settings → Hidden photos lists those groups and removes the exclusion to restore them.
- If an associated JPEG/XMP is also linked to another visible photo, the delete dialog warns
  before the shared file is hidden or trashed.

## Consequences

- Hidden assets are tied to file paths, not content. Moving a hidden file outside Lumio leaves a
  stale exclusion until the user restores it; removing a root prunes exclusions underneath it.
- Hiding a photo does not hide a different visible asset that happens to share the same stem;
  only the selected group's file paths are excluded, and shared associations are surfaced in the
  UI rather than silently following the wrong photos.
- When a catalogue arrives, hidden-asset state should migrate into the catalogue instead of
  remaining a path list in `settings.json`.
- Moves and disk deletes are the only operations that write to the filesystem. Browsing,
  scanning and thumbnail generation remain read-only apart from the cache.
