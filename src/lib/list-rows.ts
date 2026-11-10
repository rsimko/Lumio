import { assetAssociations, type AssetAssociation } from '@/lib/assets';
import type { PhotoAsset } from '@/types';

export const LIST_ROW_HEIGHT = 58;
export const LIST_ASSOCIATION_ROW_HEIGHT = 44;

export interface ListAssetRow {
  type: 'asset';
  asset: PhotoAsset;
  assetIndex: number;
  top: number;
  height: number;
}

export interface ListAssociationRow {
  type: 'association';
  asset: PhotoAsset;
  assetIndex: number;
  association: AssetAssociation;
  top: number;
  height: number;
}

export type ListRow = ListAssetRow | ListAssociationRow;

/**
 * Flattens assets and optional association rows into one virtualized list.
 * Every asset keeps a stable index used by selection, while association rows
 * inherit their parent's index and simply add height between photos.
 */
export function buildListRows(
  assets: readonly PhotoAsset[],
  showAssociations: boolean,
): ListRow[] {
  const rows: ListRow[] = [];
  let top = 0;

  assets.forEach((asset, assetIndex) => {
    rows.push({ type: 'asset', asset, assetIndex, top, height: LIST_ROW_HEIGHT });
    top += LIST_ROW_HEIGHT;

    if (!showAssociations) return;
    for (const association of assetAssociations(asset)) {
      rows.push({
        type: 'association',
        asset,
        assetIndex,
        association,
        top,
        height: LIST_ASSOCIATION_ROW_HEIGHT,
      });
      top += LIST_ASSOCIATION_ROW_HEIGHT;
    }
  });

  return rows;
}

export function listRowsHeight(rows: readonly ListRow[]): number {
  const last = rows[rows.length - 1];
  return last ? last.top + last.height : 0;
}

/** First row intersecting the viewport after applying a pixel overscan. */
export function firstVisibleListRow(
  rows: readonly ListRow[],
  scrollTop: number,
  overscan = 0,
): number {
  const threshold = Math.max(0, scrollTop - overscan);
  let low = 0;
  let high = rows.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    const row = rows[middle];
    if (row.top + row.height < threshold) low = middle + 1;
    else high = middle;
  }
  return low;
}

export function assetRowTop(rows: readonly ListRow[], assetIndex: number): number | null {
  const row = rows.find((candidate) => candidate.type === 'asset' && candidate.assetIndex === assetIndex);
  return row?.top ?? null;
}
