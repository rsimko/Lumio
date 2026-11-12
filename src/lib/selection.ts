/**
 * Pure multi-selection helpers shared by every photo layout.
 *
 * Views keep the asset order (the current scan result) and delegate awkward
 * range/toggle/reconcile rules here so keyboard and mouse behaviour stays
 * identical in grid, masonry, justified, list and filmstrip views.
 */
export interface SelectionState {
  ids: string[];
  primaryId: string | null;
  anchorId: string | null;
}

export interface SelectionModifiers {
  metaKey?: boolean;
  ctrlKey?: boolean;
  shiftKey?: boolean;
}

export function emptySelection(): SelectionState {
  return { ids: [], primaryId: null, anchorId: null };
}

export function singleSelection(id: string): SelectionState {
  return { ids: [id], primaryId: id, anchorId: id };
}

/** Toggle one item without disturbing the rest of the selection. */
export function toggleSelection(state: SelectionState, id: string): SelectionState {
  const selected = new Set(state.ids);
  if (selected.has(id)) {
    selected.delete(id);
    const ids = state.ids.filter((candidate) => selected.has(candidate));
    const primaryId =
      state.primaryId && selected.has(state.primaryId) ? state.primaryId : (ids.at(-1) ?? null);
    const anchorId = primaryId;
    return { ids, primaryId, anchorId };
  }
  selected.add(id);
  const ids = [...state.ids, id];
  return { ids, primaryId: id, anchorId: id };
}

/** Inclusive range between an anchor and a target, following the view order. */
export function rangeSelection(
  order: readonly string[],
  anchor: string,
  target: string,
): SelectionState {
  const anchorIndex = order.indexOf(anchor);
  const targetIndex = order.indexOf(target);
  if (targetIndex < 0) return emptySelection();
  if (anchorIndex < 0) return singleSelection(target);
  const start = Math.min(anchorIndex, targetIndex);
  const end = Math.max(anchorIndex, targetIndex);
  return {
    ids: order.slice(start, end + 1),
    primaryId: target,
    anchorId: anchor,
  };
}

export function selectAllSelection(order: readonly string[]): SelectionState {
  if (!order.length) return emptySelection();
  return {
    ids: [...order],
    primaryId: order[0],
    anchorId: order[0],
  };
}

/**
 * Mouse selection rules:
 * - Shift extends from the anchor (or primary when no anchor is available).
 * - Cmd/Ctrl toggles the clicked item.
 * - A plain click replaces the selection.
 */
export function clickSelection(
  order: readonly string[],
  state: SelectionState,
  id: string,
  modifiers: SelectionModifiers = {},
): SelectionState {
  if (modifiers.shiftKey) {
    const anchor =
      state.anchorId && order.includes(state.anchorId)
        ? state.anchorId
        : (state.primaryId ?? id);
    return rangeSelection(order, anchor, id);
  }
  if (modifiers.metaKey || modifiers.ctrlKey) {
    return toggleSelection(state, id);
  }
  return singleSelection(id);
}

/**
 * Keep only IDs that still exist in the current scan. When the selection is
 * empty afterwards, optionally focus the nearest surviving row so a delete or
 * move does not leave the UI without any context.
 */
export function reconcileSelection(
  order: readonly string[],
  state: SelectionState,
  focusIndex?: number | null,
): SelectionState {
  const available = new Set(order);
  const selected = new Set(state.ids);
  const ids = order.filter((id) => selected.has(id));
  if (!ids.length) {
    if (!order.length) return emptySelection();
    const index =
      focusIndex === null || focusIndex === undefined
        ? 0
        : Math.min(Math.max(0, focusIndex), order.length - 1);
    return singleSelection(order[index]);
  }
  const primaryId =
    state.primaryId && available.has(state.primaryId) ? state.primaryId : ids[0];
  const anchorId =
    state.anchorId && available.has(state.anchorId) ? state.anchorId : primaryId;
  return { ids, primaryId, anchorId };
}

/**
 * Keep selected IDs that remain visible without selecting a replacement.
 * Filtering can hide every selected photo; in that case the editor should
 * become empty rather than acting on an arbitrary survivor.
 */
export function retainVisibleSelection(
  order: readonly string[],
  state: SelectionState,
): SelectionState {
  const available = new Set(order);
  const selected = new Set(state.ids);
  const ids = order.filter((id) => selected.has(id));
  if (!ids.length) return emptySelection();
  const primaryId =
    state.primaryId && available.has(state.primaryId) ? state.primaryId : ids[0];
  const anchorId =
    state.anchorId && available.has(state.anchorId) ? state.anchorId : primaryId;
  return { ids, primaryId, anchorId };
}

/** Arrow-key navigation; Shift keeps the original anchor and extends. */
export function navigateSelection(
  order: readonly string[],
  state: SelectionState,
  offset: number,
  extend = false,
): SelectionState {
  if (!order.length) return emptySelection();
  const currentIndex = state.primaryId ? order.indexOf(state.primaryId) : -1;
  const base = currentIndex < 0 ? 0 : currentIndex;
  const nextIndex = Math.min(order.length - 1, Math.max(0, base + offset));
  const nextId = order[nextIndex];
  if (!extend) return singleSelection(nextId);
  const anchor =
    state.anchorId && order.includes(state.anchorId)
      ? state.anchorId
      : order[base];
  return rangeSelection(order, anchor, nextId);
}
