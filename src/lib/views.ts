import type { ViewMode } from '@/types';

/**
 * Layouts that share the single cycling toolbar button. Filmstrip stays a
 * separate button because it is a viewing mode rather than a browse layout.
 */
export const BROWSE_VIEW_MODES = ['grid', 'masonry', 'justified', 'list'] as const;

export type BrowseViewMode = (typeof BROWSE_VIEW_MODES)[number];

export function isBrowseViewMode(mode: ViewMode): mode is BrowseViewMode {
  return (BROWSE_VIEW_MODES as readonly ViewMode[]).includes(mode);
}

/**
 * Next browse layout in toolbar cycle order. A non-browse mode such as
 * filmstrip returns the first layout so the button always has a destination.
 */
export function nextBrowseViewMode(mode: ViewMode): BrowseViewMode {
  const index = BROWSE_VIEW_MODES.indexOf(mode as BrowseViewMode);
  if (index < 0) return BROWSE_VIEW_MODES[0];
  return BROWSE_VIEW_MODES[(index + 1) % BROWSE_VIEW_MODES.length];
}
