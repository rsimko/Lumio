import { describe, expect, it } from 'vitest';

import {
  clampAspect,
  computeGrid,
  computeJustified,
  computeMasonry,
  visibleItems,
  visibleRowRange,
} from '@/lib/layout';

describe('computeGrid', () => {
  it('fills the available width and derives the row count', () => {
    const layout = computeGrid(100, 1000, 180);
    expect(layout.columns).toBe(5);
    expect(layout.cell).toBeCloseTo(182.4, 3);
    expect(layout.rows).toBe(20);
    expect(layout.totalHeight).toBeCloseTo(3916, 3);
  });

  it('always renders at least one column', () => {
    expect(computeGrid(3, 120, 480).columns).toBe(1);
    expect(computeGrid(0, 1000, 180).rows).toBe(0);
  });
});

describe('computeJustified', () => {
  it('justifies a complete row to the available width', () => {
    const layout = computeJustified([1.5, 1.5, 1.5, 1.5, 1.5], 1000, 200);
    expect(layout.rows).toHaveLength(2);
    const first = layout.items.slice(0, 4);
    const width = first.reduce((sum, item) => sum + item.width, 0) + 3 * 12;
    expect(width).toBeCloseTo(960, 2);
    expect(layout.rows[0].height).toBeCloseTo(154, 2);
    // A trailing short row keeps the target height instead of stretching.
    expect(layout.rows[1].height).toBe(200);
  });

  it('handles an empty list', () => {
    const layout = computeJustified([], 1000, 200);
    expect(layout.items).toHaveLength(0);
    expect(layout.totalHeight).toBe(40);
  });
});

describe('computeMasonry', () => {
  it('balances items across columns without overlap', () => {
    const layout = computeMasonry([1.5, 1, 0.66, 1.5, 2, 0.8], 1000, 200);
    expect(layout.columns).toBe(5);
    expect(layout.items).toHaveLength(6);
    expect(layout.columnWidth).toBeCloseTo(182.4, 2);
    expect(layout.totalHeight).toBeGreaterThan(200);
    for (const item of layout.items) {
      expect(item.width).toBeGreaterThan(0);
      expect(item.height).toBeGreaterThan(0);
    }
  });
});

describe('visibility helpers', () => {
  it('adds overscan rows and clamps to the row count', () => {
    expect(visibleRowRange(1000, 500, 200, 20, 2)).toEqual({ start: 3, end: 10 });
    expect(visibleRowRange(0, 400, 200, 3, 2)).toEqual({ start: 0, end: 3 });
    expect(visibleRowRange(0, 400, 0, 3)).toEqual({ start: 0, end: 0 });
  });

  it('selects items intersecting the viewport', () => {
    const items = [
      { index: 0, x: 0, y: 0, width: 10, height: 100 },
      { index: 1, x: 0, y: 2000, width: 10, height: 100 },
    ];
    expect(visibleItems(items, 0, 600, 0)).toHaveLength(1);
    expect(visibleItems(items, 1900, 600, 0)).toHaveLength(1);
  });
});

describe('clampAspect', () => {
  it('guards against broken or absurd dimensions', () => {
    expect(clampAspect(0)).toBe(1.5);
    expect(clampAspect(Number.NaN)).toBe(1.5);
    expect(clampAspect(0.01)).toBe(0.32);
    expect(clampAspect(99)).toBe(3.4);
    expect(clampAspect(1.5)).toBe(1.5);
  });
});
