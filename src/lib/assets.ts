import type { PhotoAsset } from '@/types';

/** Custom MIME type used for internal photo drags. */
export const ASSET_DRAG_MIME = 'application/x-lumio-assets';

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
