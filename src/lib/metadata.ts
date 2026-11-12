import type {
  ColorLabel,
  CullFlag,
  IptcMetadata,
  MetadataSource,
  MetadataUpdate,
  PhotoAsset,
  PhotoMetadata,
} from '@/types';

export const COLOR_LABELS: readonly ColorLabel[] = [
  'red',
  'yellow',
  'green',
  'blue',
  'purple',
];

export const COLOR_LABEL_NAMES: Record<ColorLabel, string> = {
  red: 'Red',
  yellow: 'Yellow',
  green: 'Green',
  blue: 'Blue',
  purple: 'Purple',
};

/** Solid swatches used by the label picker and compact tile badges. */
export const COLOR_LABEL_SWATCHES: Record<ColorLabel, string> = {
  red: 'bg-red-500',
  yellow: 'bg-amber-400',
  green: 'bg-emerald-500',
  blue: 'bg-sky-500',
  purple: 'bg-violet-500',
};

export const COLOR_LABEL_RINGS: Record<ColorLabel, string> = {
  red: 'ring-red-500/60',
  yellow: 'ring-amber-400/60',
  green: 'ring-emerald-500/60',
  blue: 'ring-sky-500/60',
  purple: 'ring-violet-500/60',
};

export const METADATA_STARS = [1, 2, 3, 4, 5] as const;

export function emptyIptc(): IptcMetadata {
  return {
    title: '',
    description: '',
    creator: '',
    copyright: '',
    headline: '',
    credit: '',
    source: '',
    city: '',
    state: '',
    country: '',
    location: '',
    countryCode: '',
    latitude: null,
    longitude: null,
    altitude: null,
  };
}

export function emptyMetadata(): PhotoMetadata {
  return {
    rating: 0,
    label: null,
    cull: null,
    favorite: false,
    keywords: [],
    hierarchicalKeywords: [],
    iptc: emptyIptc(),
    source: 'none',
    revision: '',
    issues: [],
  };
}

export interface CommonValue<T> {
  value: T;
  mixed: boolean;
}

/**
 * Metadata shown by an editor when several photos are selected. Values that
 * differ are marked as mixed so controls can avoid pretending one photo's
 * value applies to the whole selection.
 */
export function commonMetadataValue<T>(
  assets: readonly PhotoAsset[],
  read: (metadata: PhotoMetadata) => T,
  fallback: T,
): CommonValue<T> {
  if (!assets.length) return { value: fallback, mixed: false };
  const first = read(assets[0].metadata);
  for (let index = 1; index < assets.length; index += 1) {
    if (read(assets[index].metadata) !== first) {
      return { value: fallback, mixed: true };
    }
  }
  return { value: first, mixed: false };
}

/** Clicking the active rating clears it; any other star sets it. */
export function ratingPatch(current: number, mixed: boolean, next: number): MetadataUpdate {
  if (!mixed && current === next) return { rating: 0 };
  return { rating: next };
}

/** Applies a metadata patch locally so controls respond before the write finishes. */
export function applyMetadataUpdate(
  metadata: PhotoMetadata,
  update: MetadataUpdate,
): PhotoMetadata {
  const currentIptc = metadata.iptc ?? emptyIptc();
  const patch = update.iptc;
  const iptc = patch
    ? {
        ...currentIptc,
        ...(patch.title !== undefined ? { title: patch.title } : {}),
        ...(patch.description !== undefined ? { description: patch.description } : {}),
        ...(patch.creator !== undefined ? { creator: patch.creator } : {}),
        ...(patch.copyright !== undefined ? { copyright: patch.copyright } : {}),
        ...(patch.headline !== undefined ? { headline: patch.headline } : {}),
        ...(patch.credit !== undefined ? { credit: patch.credit } : {}),
        ...(patch.source !== undefined ? { source: patch.source } : {}),
        ...(patch.city !== undefined ? { city: patch.city } : {}),
        ...(patch.state !== undefined ? { state: patch.state } : {}),
        ...(patch.country !== undefined ? { country: patch.country } : {}),
        ...(patch.location !== undefined ? { location: patch.location } : {}),
        ...(patch.countryCode !== undefined
          ? { countryCode: patch.countryCode.toUpperCase() }
          : {}),
        latitude: patch.clearLatitude ? null : (patch.latitude ?? currentIptc.latitude),
        longitude: patch.clearLongitude ? null : (patch.longitude ?? currentIptc.longitude),
        altitude: patch.clearAltitude ? null : (patch.altitude ?? currentIptc.altitude),
      }
    : currentIptc;

  return {
    ...metadata,
    rating: update.rating ?? metadata.rating,
    label: update.clearLabel ? null : (update.label ?? metadata.label),
    cull: update.clearCull ? null : (update.cull ?? metadata.cull),
    favorite: update.favorite ?? metadata.favorite,
    keywords: update.keywords ?? metadata.keywords,
    hierarchicalKeywords: update.hierarchicalKeywords ?? metadata.hierarchicalKeywords,
    iptc,
  };
}

/**
 * Clicking the active label clears it; another label replaces the selection.
 * Passing null clears the label for every selected photo.
 */
export function labelPatch(
  current: ColorLabel | null,
  mixed: boolean,
  next: ColorLabel | null,
): MetadataUpdate {
  if (next === null) return { clearLabel: true };
  if (!mixed && current === next) return { clearLabel: true };
  return { label: next };
}

/** Pick and Reject are flags; clicking the active flag clears it (Unflag). */
export function cullPatch(
  current: CullFlag | null,
  mixed: boolean,
  next: CullFlag,
): MetadataUpdate {
  if (!mixed && current === next) return { clearCull: true };
  return { cull: next };
}

/** Favoriting a mixed selection sets every photo; an all-favorite selection clears. */
export function favoritePatch(allFavorite: boolean, mixed: boolean): MetadataUpdate {
  return { favorite: mixed ? true : !allFavorite };
}

export function formatRating(rating: number): string {
  const filled = Math.max(0, Math.min(5, Math.round(rating)));
  return `${'★'.repeat(filled)}${'☆'.repeat(5 - filled)}`;
}

export function hasIptcMetadata(iptc: IptcMetadata | undefined): boolean {
  if (!iptc) return false;
  return (
    Boolean(
      iptc.title ||
        iptc.description ||
        iptc.creator ||
        iptc.copyright ||
        iptc.headline ||
        iptc.credit ||
        iptc.source ||
        iptc.city ||
        iptc.state ||
        iptc.country ||
        iptc.location ||
        iptc.countryCode,
    ) ||
    iptc.latitude !== null ||
    iptc.longitude !== null ||
    iptc.altitude !== null
  );
}

export function hasVisibleMetadata(metadata: PhotoMetadata | undefined): boolean {
  if (!metadata) return false;
  return (
    metadata.rating > 0 ||
    metadata.label !== null ||
    metadata.cull !== null ||
    metadata.favorite ||
    metadata.keywords.length > 0 ||
    metadata.hierarchicalKeywords.length > 0 ||
    hasIptcMetadata(metadata.iptc)
  );
}

export function metadataSourceLabel(source: MetadataSource): string {
  switch (source) {
    case 'sidecar':
      return 'XMP sidecar';
    case 'embedded':
      return 'Embedded XMP';
    default:
      return 'No XMP metadata';
  }
}

export function cullLabel(cull: CullFlag | null): string {
  if (cull === 'pick') return 'Picked';
  if (cull === 'reject') return 'Rejected';
  return 'Unflagged';
}

export function keywordCount(metadata: PhotoMetadata | undefined): number {
  return metadata?.keywords.length ?? 0;
}

export function hierarchicalKeywordCount(metadata: PhotoMetadata | undefined): number {
  return metadata?.hierarchicalKeywords.length ?? 0;
}

/**
 * Applies a successful write to a single asset while keeping the array order.
 * Embedded JPEG writes can change file size; sidecar writes do not touch the
 * primary file but return the same values for simplicity.
 */
export function mergeMetadataResult(
  asset: PhotoAsset,
  metadata: PhotoMetadata,
  xmpSidecar: string | null,
  fileSize: number,
  modifiedMs: number,
): PhotoAsset {
  return {
    ...asset,
    metadata,
    xmpSidecar,
    fileSize,
    modifiedMs,
  };
}
