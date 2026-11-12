import { convertFileSrc } from '@tauri-apps/api/core';
import { reactive, ref, shallowRef, watch, type Ref } from 'vue';

import { api, errorMessage } from '@/api';
import { assetPreviewKey } from '@/lib/assets';
import type { PhotoAsset, PreviewSource } from '@/types';

const EDGE_BUCKETS = [256, 512, 1024, 2048, 2560];

export function bucketEdge(requested: number): number {
  const target = Number.isFinite(requested) ? Math.max(1, requested) : 256;
  return EDGE_BUCKETS.find((edge) => edge >= target) ?? EDGE_BUCKETS[EDGE_BUCKETS.length - 1];
}

export interface ThumbEntry {
  key: string;
  assetId: string;
  edge: number;
  status: 'loading' | 'ready' | 'error';
  src: string | null;
  width: number | null;
  height: number | null;
  source: PreviewSource;
  error: string | null;
}

/** Bumped when the on-disk cache is cleared so tiles re-request previews. */
export const thumbnailEpoch = ref(0);

const entries = reactive(new Map<string, ThumbEntry>());
// Preview dimensions arrive one file at a time. Masonry and justified layouts
// depend on them, so accumulating updates and flushing them in batches keeps
// relayout work bounded instead of re-running geometry once per thumbnail.
const dimensions = new Map<string, { width: number; height: number }>();
const pendingDimensions = new Map<string, { width: number; height: number }>();
export const dimensionsVersion = ref(0);
let dimensionsTimer: number | undefined;

function queueDimensions(assetId: string, width: number, height: number): void {
  if (typeof window === 'undefined') {
    dimensions.set(assetId, { width, height });
    dimensionsVersion.value += 1;
    return;
  }
  pendingDimensions.set(assetId, { width, height });
  if (dimensionsTimer !== undefined) return;
  dimensionsTimer = window.setTimeout(() => {
    dimensionsTimer = undefined;
    for (const [key, value] of pendingDimensions) dimensions.set(key, value);
    pendingDimensions.clear();
    dimensionsVersion.value += 1;
  }, 200);
}

function devicePixelRatioValue(): number {
  if (typeof window === 'undefined') return 1;
  return Math.min(3, Math.max(1, window.devicePixelRatio || 1));
}

/** Thumbnail edge appropriate for a CSS pixel size on this display. */
export function displayEdge(cssSize: number): number {
  return bucketEdge(Math.ceil(cssSize * devicePixelRatioValue()));
}

export function thumbnailKey(asset: PhotoAsset, edge: number): string {
  return `${thumbnailEpoch.value}:${assetPreviewKey(asset)}:${edge}`;
}

export function getThumbnail(asset: PhotoAsset, edge: number): ThumbEntry {
  const bucketed = bucketEdge(edge);
  const key = thumbnailKey(asset, bucketed);
  const existing = entries.get(key);
  if (existing) return existing;

  entries.set(key, {
    key,
    assetId: asset.id,
    edge: bucketed,
    status: 'loading',
    src: null,
    width: null,
    height: null,
    source: 'placeholder',
    error: null,
  });
  const entry = entries.get(key)!;

  api
    .thumbnail(asset, bucketed)
    .then((result) => {
      if (result.width && result.height) {
        queueDimensions(asset.id, result.width, result.height);
        entry.width = result.width;
        entry.height = result.height;
      }
      entry.source = result.source;
      if (result.cachePath) {
        entry.src = convertFileSrc(result.cachePath);
        entry.status = 'ready';
      } else {
        entry.status = 'error';
        entry.error = result.error ?? 'No preview is available for this file.';
      }
    })
    .catch((error: unknown) => {
      // Entries can outlive a cache reset; only mutate the one we created.
      if (entries.get(key) !== entry) return;
      entry.status = 'error';
      entry.error = errorMessage(error);
    });

  return entry;
}

/**
 * Downgrade an entry when the webview cannot decode the cached preview (for
 * example a deleted cache file, a denied asset-protocol read, or a corrupt
 * JPEG). Treating it like a generation error makes every view fall back to the
 * placeholder instead of showing a broken-image glyph.
 */
export function markPreviewFailed(entry: ThumbEntry | null | undefined): void {
  if (!entry) return;
  entry.status = 'error';
  entry.error = 'The cached preview could not be loaded.';
  entry.src = null;
}

/** Preview pixel dimensions reported by the backend (orientation applied). */
export function dimensionsFor(asset: PhotoAsset): { width: number; height: number } | null {
  // Track the batched dimension version so computed layouts reflow when new
  // previews report their true aspect ratio.
  void dimensionsVersion.value;
  const known = dimensions.get(asset.id);
  if (known) return known;
  if (asset.width && asset.height) {
    return { width: asset.width, height: asset.height };
  }
  return null;
}

export function aspectFor(asset: PhotoAsset): number {
  const known = dimensionsFor(asset);
  if (!known) return 1.5;
  return known.width / known.height;
}

export function resetThumbnails(): void {
  thumbnailEpoch.value += 1;
  entries.clear();
  dimensions.clear();
  pendingDimensions.clear();
  if (dimensionsTimer !== undefined) {
    window.clearTimeout(dimensionsTimer);
    dimensionsTimer = undefined;
  }
  dimensionsVersion.value += 1;
}

/**
 * Reactive thumbnail for one photo at approximately `size` CSS pixels. The
 * backend returns camera preview dimensions so layout can correct estimated
 * aspect ratios as previews arrive.
 */
export function useThumbnail(
  asset: () => PhotoAsset | null | undefined,
  size: () => number,
): Ref<ThumbEntry | null> {
  const entry = shallowRef<ThumbEntry | null>(null);

  const load = () => {
    const current = asset();
    if (!current) {
      entry.value = null;
      return;
    }
    entry.value = getThumbnail(current, displayEdge(size()));
  };

  watch([asset, size, thumbnailEpoch], load, { immediate: true });
  return entry;
}
