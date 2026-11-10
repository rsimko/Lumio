import { onBeforeUnmount, onMounted, ref, type Ref } from 'vue';

export interface ScrollMetrics {
  width: Ref<number>;
  height: Ref<number>;
  scrollTop: Ref<number>;
  scrollLeft: Ref<number>;
}

/** Tracks the viewport size and scroll offset of a scrollable element. */
export function useScrollMetrics(element: Ref<HTMLElement | null>): ScrollMetrics {
  const width = ref(0);
  const height = ref(0);
  const scrollTop = ref(0);
  const scrollLeft = ref(0);
  let observer: ResizeObserver | null = null;
  let frame = 0;

  const measure = () => {
    const el = element.value;
    if (!el) return;
    width.value = el.clientWidth;
    height.value = el.clientHeight;
  };

  const flush = () => {
    frame = 0;
    const el = element.value;
    if (!el) return;
    scrollTop.value = el.scrollTop;
    scrollLeft.value = el.scrollLeft;
  };

  const onScroll = () => {
    if (frame) return;
    frame = requestAnimationFrame(flush);
  };

  onMounted(() => {
    const el = element.value;
    if (!el) return;
    measure();
    observer = new ResizeObserver(measure);
    observer.observe(el);
    el.addEventListener('scroll', onScroll, { passive: true });
  });

  onBeforeUnmount(() => {
    observer?.disconnect();
    element.value?.removeEventListener('scroll', onScroll);
    if (frame) cancelAnimationFrame(frame);
  });

  return { width, height, scrollTop, scrollLeft };
}
