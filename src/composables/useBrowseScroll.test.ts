import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { describe, expect, it } from 'vitest';

import { useBrowseScroll } from '@/composables/useBrowseScroll';
import type { ScrollMetrics } from '@/composables/useScrollMetrics';

function harness(folder: string) {
  const scroller = ref<HTMLElement | null>(null);
  const folderName = ref(folder);
  const metrics: ScrollMetrics = {
    width: ref(0),
    height: ref(0),
    scrollTop: ref(0),
    scrollLeft: ref(0),
  };
  const Component = defineComponent({
    setup() {
      useBrowseScroll(scroller, metrics, () => folderName.value);
      return () => h('div', { ref: scroller });
    },
  });
  const wrapper = mount(Component, { attachTo: document.body });
  const element = wrapper.element as HTMLElement;

  Object.defineProperty(element, 'scrollHeight', { configurable: true, value: 2000 });
  Object.defineProperty(element, 'clientHeight', { configurable: true, value: 400 });

  async function measure(scrollTop = 0): Promise<void> {
    element.scrollTop = scrollTop;
    metrics.scrollTop.value = scrollTop;
    metrics.width.value = 800;
    metrics.height.value = 400;
    await nextTick();
  }

  return { wrapper, element, metrics, folderName, measure };
}

describe('browse scroll sharing', () => {
  it('restores the relative position when another layout mounts', async () => {
    const first = harness('shared-folder');
    await first.measure();
    first.element.scrollTop = 480;
    first.metrics.scrollTop.value = 480;
    await nextTick();
    first.wrapper.unmount();

    const second = harness('shared-folder');
    await second.measure();
    expect(second.element.scrollTop).toBe(480);
    second.wrapper.unmount();
  });

  it('starts at the top for a different folder and ignores stale unmounts', async () => {
    const first = harness('folder-a');
    await first.measure();
    first.element.scrollTop = 960;
    first.metrics.scrollTop.value = 960;
    await nextTick();

    // The folder changes before the view unmounts; its stale offset must not
    // leak into the new folder.
    first.folderName.value = 'folder-b';
    first.wrapper.unmount();
    const second = harness('folder-b');
    await second.measure();
    expect(second.element.scrollTop).toBe(0);
    second.wrapper.unmount();
  });
});
