import type { PhotoAsset } from '@/types';

/** Custom MIME type used for internal photo drags. */
export const ASSET_DRAG_MIME = 'application/x-lumio-assets';

export interface AssetAssociation {
  path: string;
  kind: 'jpeg' | 'xmp';
  extra: boolean;
}

/**
 * Associated files in display order, with duplicates and the primary file
 * removed. Extra sidecars are populated defensively by the scanner, so the
 * primary sidecar can appear twice; the list view must only show it once.
 */
export function assetAssociations(asset: PhotoAsset): AssetAssociation[] {
  const seen = new Set<string>([asset.primaryPath]);
  const associations: AssetAssociation[] = [];
  const add = (path: string | null, kind: AssetAssociation['kind'], extra: boolean) => {
    if (!path || seen.has(path)) return;
    seen.add(path);
    associations.push({ path, kind, extra });
  };

  add(asset.companionJpeg, 'jpeg', false);
  for (const path of asset.extraCompanions) add(path, 'jpeg', true);
  add(asset.xmpSidecar, 'xmp', false);
  for (const path of asset.extraSidecars) add(path, 'xmp', true);
  return associations;
}

/**
 * Inputs that decide which file backs a preview. The value changes when a
 * rescan discovers or removes a companion JPEG, so thumbnail caches can be
 * invalidated without restarting the app.
 */
export function assetPreviewKey(asset: PhotoAsset): string {
  return `${asset.id}:${asset.previewHint}:${asset.companionJpeg ?? ''}`;
}

/** Every file that belongs to one photo, in primary-first order. */
export function assetFilePaths(asset: PhotoAsset): string[] {
  const paths = [
    asset.primaryPath,
    asset.companionJpeg,
    ...asset.extraCompanions,
    asset.xmpSidecar,
    ...asset.extraSidecars,
  ].filter((path): path is string => Boolean(path));
  return [...new Set(paths)];
}

/** Deduplicated file paths across a batch of photos. */
export function collectAssetFilePaths(assets: readonly PhotoAsset[]): string[] {
  const paths = new Set<string>();
  for (const asset of assets) {
    for (const path of assetFilePaths(asset)) paths.add(path);
  }
  return [...paths];
}

/**
 * Files that are associated with a photo outside the operation batch. These
 * are shown as a warning before hiding or trashing shared JPG/XMP files.
 */
export function sharedAssociationPaths(
  assets: readonly PhotoAsset[],
  targets: readonly PhotoAsset[],
): string[] {
  const targetIds = new Set(targets.map((asset) => asset.id));
  const targetPaths = new Set(collectAssetFilePaths(targets));
  const shared = new Set<string>();
  for (const asset of assets) {
    if (targetIds.has(asset.id)) continue;
    for (const path of assetFilePaths(asset)) {
      if (targetPaths.has(path)) shared.add(path);
    }
  }
  return [...shared];
}
