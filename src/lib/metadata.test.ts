import { describe, expect, it } from 'vitest';

import {
  commonMetadataValue,
  cullPatch,
  favoritePatch,
  formatRating,
  hasVisibleMetadata,
  labelPatch,
  ratingPatch,
} from '@/lib/metadata';
import type { PhotoAsset, PhotoMetadata } from '@/types';

function metadata(patch: Partial<PhotoMetadata> = {}): PhotoMetadata {
  return {
    rating: 0,
    label: null,
    cull: null,
    favorite: false,
    keywords: [],
    hierarchicalKeywords: [],
    source: 'sidecar',
    revision: 'revision',
    issues: [],
    ...patch,
  };
}

function asset(id: string, patch: Partial<PhotoMetadata> = {}): PhotoAsset {
  return {
    id,
    primaryPath: `/photos/${id}.dng`,
    fileName: `${id}.dng`,
    stem: id,
    extension: 'dng',
    kind: 'raw',
    companionJpeg: null,
    xmpSidecar: `/photos/${id}.dng.xmp`,
    extraSidecars: [],
    extraCompanions: [],
    previewHint: 'embeddedRaw',
    fileSize: 1,
    modifiedMs: 0,
    width: null,
    height: null,
    relativeDir: '',
    metadata: metadata(patch),
  };
}

describe('metadata selection helpers', () => {
  it('reports a shared value for a homogeneous selection', () => {
    const assets = [asset('a', { rating: 4 }), asset('b', { rating: 4 })];
    expect(commonMetadataValue(assets, (value) => value.rating, 0)).toEqual({
      value: 4,
      mixed: false,
    });
  });

  it('reports mixed values and falls back to a neutral value', () => {
    const assets = [asset('a', { label: 'red' }), asset('b', { label: 'blue' })];
    expect(commonMetadataValue(assets, (value) => value.label, null)).toEqual({
      value: null,
      mixed: true,
    });
  });

  it('handles an empty selection', () => {
    expect(commonMetadataValue([], (value) => value.rating, 0)).toEqual({
      value: 0,
      mixed: false,
    });
  });
});

describe('metadata patches', () => {
  it('sets ratings and clears the active rating on a second click', () => {
    expect(ratingPatch(0, false, 3)).toEqual({ rating: 3 });
    expect(ratingPatch(3, false, 3)).toEqual({ rating: 0 });
    expect(ratingPatch(0, true, 3)).toEqual({ rating: 3 });
  });

  it('sets labels and clears the active label on a second click', () => {
    expect(labelPatch(null, false, 'green')).toEqual({ label: 'green' });
    expect(labelPatch('green', false, 'green')).toEqual({ clearLabel: true });
    expect(labelPatch(null, true, 'green')).toEqual({ label: 'green' });
    expect(labelPatch('green', false, null)).toEqual({ clearLabel: true });
    expect(labelPatch(null, true, null)).toEqual({ clearLabel: true });
  });

  it('sets cull flags and clears the active flag', () => {
    expect(cullPatch(null, false, 'pick')).toEqual({ cull: 'pick' });
    expect(cullPatch('pick', false, 'pick')).toEqual({ clearCull: true });
    expect(cullPatch('reject', false, 'pick')).toEqual({ cull: 'pick' });
  });

  it('toggles favorites for a mixed selection by setting all', () => {
    expect(favoritePatch(false, false)).toEqual({ favorite: true });
    expect(favoritePatch(true, false)).toEqual({ favorite: false });
    expect(favoritePatch(false, true)).toEqual({ favorite: true });
  });

  it('formats stars and detects visible metadata', () => {
    expect(formatRating(3)).toBe('★★★☆☆');
    expect(formatRating(9)).toBe('★★★★★');
    expect(hasVisibleMetadata(metadata())).toBe(false);
    expect(hasVisibleMetadata(metadata({ favorite: true }))).toBe(true);
    expect(hasVisibleMetadata(metadata({ keywords: ['one'] }))).toBe(true);
  });
});
