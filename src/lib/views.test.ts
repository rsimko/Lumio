import { describe, expect, it } from 'vitest';

import { BROWSE_VIEW_MODES, isBrowseViewMode, nextBrowseViewMode } from '@/lib/views';

describe('browse view cycling', () => {
  it('recognises only the grouped browse layouts', () => {
    expect(BROWSE_VIEW_MODES).toEqual(['grid', 'masonry', 'justified', 'list']);
    expect(isBrowseViewMode('grid')).toBe(true);
    expect(isBrowseViewMode('list')).toBe(true);
    expect(isBrowseViewMode('filmstrip')).toBe(false);
  });

  it('cycles through every layout and wraps around', () => {
    expect(nextBrowseViewMode('grid')).toBe('masonry');
    expect(nextBrowseViewMode('masonry')).toBe('justified');
    expect(nextBrowseViewMode('justified')).toBe('list');
    expect(nextBrowseViewMode('list')).toBe('grid');
  });

  it('returns to the first layout when leaving filmstrip', () => {
    expect(nextBrowseViewMode('filmstrip')).toBe('grid');
  });
});
