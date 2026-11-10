# ADR 0003 — JSON settings instead of SQLite in Milestone 1

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Lumio will eventually need an indexed catalogue for search, ratings, labels and fast
startup across very large libraries. Milestone 1 only needs to remember a list of roots
and a small set of UI preferences. Introducing a database schema now would lock in
decisions before the metadata model exists.

## Decision

Persist M1 state as a single JSON file: `settings.json` in the application config
directory. Writes are atomic: serialise to a temporary file in the same directory,
`fsync`, then rename over the target. Version the envelope so a future migration is
possible.

If the file is corrupt, preserve it as `settings.json.corrupt-<timestamp>` and start with
an empty library, reporting a recoverable warning to the UI. Never silently discard user
configuration.

A thumbnail cache is separate and rebuildable; it is keyed by content identity
(path + size + mtime) and can be deleted at any time.

## Consequences

- M1 has no schema migrations, no database dependency and no background indexing.
- Scanning reads the filesystem directly on every folder open.
- A later milestone will add a rebuildable SQLite (or similar) catalogue; the JSON file
  remains only for UI preferences and roots.
