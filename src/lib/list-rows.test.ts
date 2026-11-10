import { describe, expect, it } from 'vitest';

import {
  assetRowTop,
  buildListRows,
  firstVisibleListRow,
  LIST_ASSOCIATION_ROW_HEIGHT,
  LIST_ROW_HEIGHT,
  listRowsHeight,
} from '@/lib/list-rows';
import type { PhotoAsset } from '@/types';

function asset(overrides: Partial<PhotoAsset> & Pick<PhotoAsset, 'id' | 'primaryPath'>): PhotoAsset {
  const fileName = overrides.primaryPath.split('/').pop() ?? 'IMG_0001.CR2';
  return {
    fileName,
    stem: fileName.replace(/\.[^.]+$/, ''),
    extension: 'cr2',
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
    ...overrides,
  };
}

const raw1 = asset({
  id: 'a',
  primaryPath: '/photos/IMG_0001.CR2',
  companionJpeg: '/photos/IMG_0001.jpg',
  xmpSidecar: '/photos/IMG_0001.CR2.xmp',
});
const raw2 = asset({ id: 'b', primaryPath: '/photos/IMG_0002.CR2' });

describe('list row layout', () => {
  it('keeps one row per asset when associations are hidden', () => {
    const rows = buildListRows([raw1, raw2], false);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ type: 'asset', assetIndex: 0, top: 0, height: LIST_ROW_HEIGHT });
    expect(rows[1]).toMatchObject({
      type: 'asset',
      assetIndex: 1,
      top: LIST_ROW_HEIGHT,
      height: LIST_ROW_HEIGHT,
    });
    expect(listRowsHeight(rows)).toBe(LIST_ROW_HEIGHT * 2);
  });

  it('inserts association rows beneath their raw and before the next asset', () => {
    const rows = buildListRows([raw1, raw2], true);
    expect(rows.map((row) => row.type)).toEqual(['asset', 'association', 'association', 'asset']);
    expect(rows[0].top).toBe(0);
    expect(rows[1].top).toBe(LIST_ROW_HEIGHT);
    expect(rows[2].top).toBe(LIST_ROW_HEIGHT + LIST_ASSOCIATION_ROW_HEIGHT);
    expect(rows[3].top).toBe(
      LIST_ROW_HEIGHT + LIST_ASSOCIATION_ROW_HEIGHT * 2,
    );
    expect(assetRowTop(rows, 0)).toBe(0);
    expect(assetRowTop(rows, 1)).toBe(rows[3].top);
    expect(listRowsHeight(rows)).toBe(
      LIST_ROW_HEIGHT * 2 + LIST_ASSOCIATION_ROW_HEIGHT * 2,
    );
  });

  it('finds the first row in view with a pixel overscan', () => {
    const rows = buildListRows([raw1, raw2], true);
    expect(firstVisibleListRow(rows, 150, 0)).toBe(3);
    expect(firstVisibleListRow(rows, 150, 60)).toBe(1);
    expect(firstVisibleListRow([], 150, 60)).toBe(0);
  });
});
