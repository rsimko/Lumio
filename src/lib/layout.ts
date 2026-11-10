/**
 * Pure geometry helpers for the photo views. Keeping these free of Vue and
 * DOM state makes the layout behaviour unit-testable and predictable.
 */

export interface PositionedItem {
  index: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RowInfo {
  y: number;
  height: number;
  startIndex: number;
  endIndex: number;
}

export interface GridLayout {
  padding: number;
  gap: number;
  columns: number;
  cell: number;
  rowStride: number;
  rows: number;
  totalHeight: number;
}

export interface JustifiedLayout {
  items: PositionedItem[];
  rows: RowInfo[];
  totalHeight: number;
}

export interface MasonryLayout {
  items: PositionedItem[];
  columns: number;
  columnWidth: number;
  totalHeight: number;
}

export const DEFAULT_PADDING = 20;
export const DEFAULT_GAP = 12;

export function clampAspect(aspect: number): number {
  if (!Number.isFinite(aspect) || aspect <= 0) return 1.5;
  return Math.min(3.4, Math.max(0.32, aspect));
}

export function computeGrid(
  count: number,
  width: number,
  targetCell: number,
  padding = DEFAULT_PADDING,
  gap = DEFAULT_GAP,
): GridLayout {
  const available = Math.max(120, width - padding * 2);
  const cellTarget = Math.min(480, Math.max(100, targetCell));
  const columns = Math.max(1, Math.floor((available + gap) / (cellTarget + gap)));
  const cell = (available - gap * (columns - 1)) / columns;
  const rows = Math.ceil(Math.max(0, count) / columns);
  const totalHeight =
    padding * 2 + rows * cell + Math.max(0, rows - 1) * gap;
  return { padding, gap, columns, cell, rowStride: cell + gap, rows, totalHeight };
}

export function computeJustified(
  aspects: number[],
  width: number,
  targetRowHeight: number,
  padding = DEFAULT_PADDING,
  gap = DEFAULT_GAP,
): JustifiedLayout {
  const available = Math.max(160, width - padding * 2);
  const target = Math.min(420, Math.max(110, targetRowHeight));
  const items: PositionedItem[] = [];
  const rows: RowInfo[] = [];
  let index = 0;
  let y = padding;

  while (index < aspects.length) {
    const startIndex = index;
    let aspectSum = 0;
    let rowCount = 0;
    while (index < aspects.length) {
      aspectSum += clampAspect(aspects[index]);
      rowCount += 1;
      index += 1;
      if (aspectSum * target + gap * (rowCount - 1) >= available) break;
    }

    const isLastRow = index >= aspects.length;
    let height = (available - gap * (rowCount - 1)) / aspectSum;
    if (isLastRow) {
      height = target;
    } else {
      // Guard against a pathological row (one very wide panorama) becoming
      // taller than the configured target.
      height = Math.min(height, target * 1.6);
    }

    let x = padding;
    for (let cursor = startIndex; cursor < index; cursor += 1) {
      const itemWidth = clampAspect(aspects[cursor]) * height;
      items.push({
        index: cursor,
        x,
        y,
        width: itemWidth,
        height,
      });
      x += itemWidth + gap;
    }
    rows.push({ y, height, startIndex, endIndex: index });
    y += height + gap;
  }

  const totalHeight =
    rows.length > 0
      ? padding * 2 +
        rows.reduce((sum, row) => sum + row.height, 0) +
        gap * (rows.length - 1)
      : padding * 2;

  return { items, rows, totalHeight };
}

export function computeMasonry(
  aspects: number[],
  width: number,
  targetColumnWidth: number,
  padding = DEFAULT_PADDING,
  gap = DEFAULT_GAP,
): MasonryLayout {
  const available = Math.max(120, width - padding * 2);
  const target = Math.min(480, Math.max(100, targetColumnWidth));
  const columns = Math.max(1, Math.round((available + gap) / (target + gap)));
  const columnWidth = (available - gap * (columns - 1)) / columns;
  const heights = new Array<number>(columns).fill(padding);
  const items: PositionedItem[] = [];

  for (let index = 0; index < aspects.length; index += 1) {
    let column = 0;
    for (let candidate = 1; candidate < columns; candidate += 1) {
      if (heights[candidate] < heights[column] - 0.01) column = candidate;
    }
    const aspect = clampAspect(aspects[index]);
    const height = Math.max(48, columnWidth / aspect);
    items.push({
      index,
      x: padding + column * (columnWidth + gap),
      y: heights[column],
      width: columnWidth,
      height,
    });
    heights[column] += height + gap;
  }

  const tallest = heights.length ? Math.max(...heights) : padding;
  const totalHeight =
    items.length > 0 ? tallest - gap + padding : padding * 2;

  return { items, columns, columnWidth, totalHeight };
}

/** Inclusive-start, exclusive-end range of visible rows. */
export function visibleRowRange(
  scrollTop: number,
  viewportHeight: number,
  rowStride: number,
  rowCount: number,
  overscan = 2,
): { start: number; end: number } {
  if (rowStride <= 0 || rowCount <= 0) return { start: 0, end: 0 };
  const start = Math.max(0, Math.floor(scrollTop / rowStride) - overscan);
  const end = Math.min(
    rowCount,
    Math.ceil((scrollTop + viewportHeight) / rowStride) + overscan,
  );
  return { start, end };
}

/** Items whose vertical extent intersects the viewport, with overscan in px. */
export function visibleItems(
  items: PositionedItem[],
  scrollTop: number,
  viewportHeight: number,
  overscan = 400,
): PositionedItem[] {
  const top = scrollTop - overscan;
  const bottom = scrollTop + viewportHeight + overscan;
  return items.filter((item) => item.y + item.height >= top && item.y <= bottom);
}
