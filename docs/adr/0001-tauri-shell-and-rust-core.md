# ADR 0001 — Tauri 2 shell with a Tauri-free Rust core

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Lumio must read large RAW files, walk folder trees, extract embedded previews and cache
thumbnails. Those operations are CPU- and IO-heavy, need to run off the UI thread, and
must be testable without launching a desktop window. The UI needs a modern, reactive
component model and fast iteration.

## Decision

Split the project into:

- `crates/lumio-core` — pure Rust domain logic: config, scanning, association, preview
  extraction, thumbnail generation. No Tauri types, tested with `cargo test`.
- `src-tauri` — a thin Tauri 2 shell. It resolves application paths, owns shared state,
  validates webview-supplied paths and exposes commands. Blocking work runs in
  `spawn_blocking`.
- `src` — a Vue 3 + TypeScript + Vite frontend using Pinia and Tailwind CSS 3.4.

Tauri 2 is used instead of Electron because it ships a small native shell, has a
first-class Rust command layer and uses the system WebView. Tailwind 3.4 is pinned rather
than 4.x for older WebKit compatibility on the supported OS floor.

## Consequences

- Domain behaviour can be unit-tested headlessly and reused by future frontends.
- The Tauri layer must stay deliberately thin; business logic belongs in `lumio-core`.
- Two test runners exist: `cargo test` and `vitest`.
