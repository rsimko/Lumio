import { describe, expect, it } from 'vitest';

import {
  activeFilterCount,
  emptyFilters,
  filterAssets,
  hasActiveFilters,
  labelFilterName,
  matchesFilters,
  toggleFilterValue,
  type AssetFilters,
} from '@/lib/filters';
import { emptyMetadata } from '@/lib/metadata';
import type { PhotoAsset, PhotoMetadata } from '@/types';

function asset(id: string, patch: Partial<PhotoMetadata> = {}): PhotoAsset {
  return {
    id,
    primaryPath: `/photos/${id}.dng`,
    fileName: `${id}.dng`,
    stem: id,
    extension: 'dng',
    kind: 'raw',
    companionJpeg: null,
    xmpSidecar: null,
    extraSidecars: [],
    extraCompanions: [],
    previewHint: 'embeddedRaw',
    fileSize: 1,
    modifiedMs: 0,
    width: null,
    height: null,
    relativeDir: '',
    metadata: { ...emptyMetadata(), ...patch },
  };
}

const assets = [
  asset('a', { rating: 3, label: 'red', cull: 'pick', favorite: true }),
  asset('b', { rating: 3, label: 'blue' }),
  asset('c', { rating: 5, label: 'red', cull: 'reject' }),
  asset('d'),
];

function filters(patch: Partial<AssetFilters>): AssetFilters {
  return { ...emptyFilters(), ...patch };
}

describe('filter helpers', () => {
  it('matches everything when no filters are active', () => {
    expect(hasActiveFilters(emptyFilters())).toBe(false);
    expect(activeFilterCount(emptyFilters())).toBe(0);
    expect(filterAssets(assets, emptyFilters())).toEqual(assets);
  });

  it('ORs values inside the rating group', () => {
    const selected = filters({ ratings: [3, 5] });
    expect(filterAssets(assets, selected).map((photo) => photo.id)).toEqual(['a', 'b', 'c']);
  });

  it('filters unrated photos', () => {
    const selected = filters({ ratings: [0] });
    expect(filterAssets(assets, selected).map((photo) => photo.id)).toEqual(['d']);
  });

  it('ORs label values and handles unlabeled photos', () => {
    expect(filterAssets(assets, filters({ labels: ['red', 'unlabeled'] })).map((photo) => photo.id)).toEqual([
      'a',
      'c',
      'd',
    ]);
    expect(labelFilterName('unlabeled')).toBe('Unlabeled');
    expect(labelFilterName('red')).toBe('Red');
  });

  it('ORs culling values and handles unflagged photos', () => {
    expect(filterAssets(assets, filters({ culls: ['pick', 'unflagged'] })).map((photo) => photo.id)).toEqual([
      'a',
      'b',
      'd',
    ]);
  });

  it('matches favorite and not-favorite photos', () => {
    expect(filterAssets(assets, filters({ favorites: ['favorite'] })).map((photo) => photo.id)).toEqual([
      'a',
    ]);
    expect(
      filterAssets(assets, filters({ favorites: ['notFavorite'] })).map((photo) => photo.id),
    ).toEqual(['b', 'c', 'd']);
  });

  it('ANDs different filter groups', () => {
    const selected = filters({ ratings: [3], labels: ['red'], culls: ['pick'] });
    expect(filterAssets(assets, selected).map((photo) => photo.id)).toEqual(['a']);
    expect(matchesFilters(assets[1], selected)).toBe(false);
  });

  it('toggles values and counts active chips', () => {
    expect(toggleFilterValue([], 'red')).toEqual(['red']);
    expect(toggleFilterValue(['red'], 'red')).toEqual([]);
    expect(activeFilterCount(filters({ ratings: [3, 4], labels: ['red'], favorites: ['favorite'] }))).toBe(
      4,
    );
    expect(hasActiveFilters(filters({ culls: ['pick'] }))).toBe(true);
  });
});
