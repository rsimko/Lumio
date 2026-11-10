import { describe, expect, it } from 'vitest';

import {
  clickSelection,
  emptySelection,
  navigateSelection,
  rangeSelection,
  reconcileSelection,
  selectAllSelection,
  singleSelection,
  toggleSelection,
} from '@/lib/selection';

const order = ['a', 'b', 'c', 'd', 'e'];

describe('selection helpers', () => {
  it('selects one item on a plain click', () => {
    expect(clickSelection(order, selectAllSelection(order), 'c')).toEqual({
      ids: ['c'],
      primaryId: 'c',
      anchorId: 'c',
    });
  });

  it('toggles items with Cmd/Ctrl and keeps the remaining primary', () => {
    const first = toggleSelection(singleSelection('b'), 'd');
    expect(first.ids).toEqual(['b', 'd']);
    expect(first.primaryId).toBe('d');

    const second = toggleSelection(first, 'd');
    expect(second.ids).toEqual(['b']);
    expect(second.primaryId).toBe('b');
  });

  it('extends a range from the anchor in either direction', () => {
    expect(rangeSelection(order, 'b', 'd').ids).toEqual(['b', 'c', 'd']);
    expect(rangeSelection(order, 'd', 'b').ids).toEqual(['b', 'c', 'd']);
  });

  it('uses the anchor for shift-click ranges', () => {
    const state = singleSelection('b');
    const extended = clickSelection(order, state, 'd', { shiftKey: true });
    expect(extended.ids).toEqual(['b', 'c', 'd']);
    expect(extended.anchorId).toBe('b');
    expect(extended.primaryId).toBe('d');
  });

  it('selects all and clears', () => {
    expect(selectAllSelection(order).ids).toEqual(order);
    expect(selectAllSelection([])).toEqual(emptySelection());
  });

  it('reconciles missing items and can focus the nearest survivor', () => {
    const state = { ids: ['a', 'c'], primaryId: 'c', anchorId: 'a' };
    expect(reconcileSelection(['a', 'b'], state).ids).toEqual(['a']);
    expect(reconcileSelection([], state)).toEqual(emptySelection());

    const gone = { ids: ['x'], primaryId: 'x', anchorId: 'x' };
    expect(reconcileSelection(order, gone, 2)).toEqual(singleSelection('c'));
  });

  it('navigates and extends with shift-arrow', () => {
    const state = singleSelection('b');
    expect(navigateSelection(order, state, 1)).toEqual(singleSelection('c'));

    const extended = navigateSelection(order, state, 2, true);
    expect(extended.ids).toEqual(['b', 'c', 'd']);
    expect(extended.primaryId).toBe('d');
    expect(extended.anchorId).toBe('b');
  });
});
