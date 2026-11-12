import { onBeforeUnmount, watch, type Ref } from 'vue';

import type { ScrollMetrics } from '@/composables/useScrollMetrics';

interface BrowseScrollState {
  folder: string | null;
  fraction: number;
}

// Browse layouts share one relative scroll position so switching between
// grid, masonry, justified and list keeps the viewer near the same photos.
const state: BrowseScrollState = { folder: null, fraction: 0 };

function scrollFraction(element: HTMLElement): number {
  const range = element.scrollHeight - element.clientHeight;
  if (range <= 0) return 0;
  return Math.min(1, Math.max(0, element.scrollTop / range));
}

/**
 * Remembers the scroll position of the current folder as a fraction of the
 * scrollable range and restores it when another browse layout mounts.
 */
export function useBrowseScroll(
  scroller: Ref<HTMLElement | null>,
  metrics: ScrollMetrics,
  folder: () => string | null,
): void {
  const mountedFolder = folder();
  let ready = false;

  const remember = () => {
    const element = scroller.value;
    if (!ready || !element) return;
    // A folder change unmounts this view; ignore the stale scroll offset.
    if (folder() !== mountedFolder) return;
    state.folder = mountedFolder;
    state.fraction = scrollFraction(element);
  };

  watch(metrics.scrollTop, remember);

  watch(
    [metrics.width, metrics.height],
    () => {
      if (ready) return;
      const element = scroller.value;
      if (!element || metrics.width.value <= 0 || metrics.height.value <= 0) return;
      ready = true;

      const currentFolder = folder();
      if (state.folder !== currentFolder) {
        state.folder = currentFolder;
        state.fraction = 0;
      }
      if (state.fraction <= 0) return;

      const range = element.scrollHeight - element.clientHeight;
      if (range <= 0) return;
      element.scrollTop = state.fraction * range;
      metrics.scrollTop.value = element.scrollTop;
    },
    { flush: 'post' },
  );

  onBeforeUnmount(remember);
}
