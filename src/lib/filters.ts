import { COLOR_LABEL_NAMES } from '@/lib/metadata';
import type { ColorLabel, CullFlag, PhotoAsset } from '@/types';

export type RatingFilterValue = 0 | 1 | 2 | 3 | 4 | 5;
export type LabelFilterValue = ColorLabel | 'unlabeled';
export type CullFilterValue = CullFlag | 'unflagged';
export type FavoriteFilterValue = 'favorite' | 'notFavorite';

/** Where the active filters look for photos. */
export type FilterScope = 'folder' | 'library';

/** Filters for the current folder. Values inside a group match any value. */
export interface AssetFilters {
  ratings: RatingFilterValue[];
  labels: LabelFilterValue[];
  culls: CullFilterValue[];
  favorites: FavoriteFilterValue[];
}

export const RATING_FILTER_VALUES: readonly RatingFilterValue[] = [0, 1, 2, 3, 4, 5];

export const LABEL_FILTER_VALUES: readonly LabelFilterValue[] = [
  'red',
  'yellow',
  'green',
  'blue',
  'purple',
  'unlabeled',
];

export const CULL_FILTER_VALUES: readonly CullFilterValue[] = ['pick', 'reject', 'unflagged'];

export const FAVORITE_FILTER_VALUES: readonly FavoriteFilterValue[] = [
  'favorite',
  'notFavorite',
];

export function emptyFilters(): AssetFilters {
  return { ratings: [], labels: [], culls: [], favorites: [] };
}

export function hasActiveFilters(filters: AssetFilters): boolean {
  return (
    filters.ratings.length > 0 ||
    filters.labels.length > 0 ||
    filters.culls.length > 0 ||
    filters.favorites.length > 0
  );
}

/** Number of selected filter chips, used by the rail badge. */
export function activeFilterCount(filters: AssetFilters): number {
  return (
    filters.ratings.length +
    filters.labels.length +
    filters.culls.length +
    filters.favorites.length
  );
}

/** Adds or removes one value while keeping the existing order. */
export function toggleFilterValue<T>(values: readonly T[], value: T): T[] {
  return values.includes(value)
    ? values.filter((candidate) => candidate !== value)
    : [...values, value];
}

function matchesFavorite(asset: PhotoAsset, value: FavoriteFilterValue): boolean {
  return value === 'favorite' ? asset.metadata.favorite : !asset.metadata.favorite;
}

/**
 * All active groups must match, while any selected value inside a group is
 * enough. This makes "3 stars + red" mean both attributes, but "3 + 4 stars"
 * means either rating.
 */
export function matchesFilters(asset: PhotoAsset, filters: AssetFilters): boolean {
  if (
    filters.ratings.length > 0 &&
    !filters.ratings.includes(asset.metadata.rating as RatingFilterValue)
  ) {
    return false;
  }

  const label = asset.metadata.label ?? 'unlabeled';
  if (filters.labels.length > 0 && !filters.labels.includes(label)) {
    return false;
  }

  const cull = asset.metadata.cull ?? 'unflagged';
  if (filters.culls.length > 0 && !filters.culls.includes(cull)) {
    return false;
  }

  if (
    filters.favorites.length > 0 &&
    !filters.favorites.some((value) => matchesFavorite(asset, value))
  ) {
    return false;
  }

  return true;
}

export function filterAssets(
  assets: readonly PhotoAsset[],
  filters: AssetFilters,
): PhotoAsset[] {
  if (!hasActiveFilters(filters)) return assets as PhotoAsset[];
  return assets.filter((asset) => matchesFilters(asset, filters));
}

export function labelFilterName(value: LabelFilterValue): string {
  return value === 'unlabeled' ? 'Unlabeled' : COLOR_LABEL_NAMES[value];
}
