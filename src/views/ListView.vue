<script setup lang="ts">
import { Paperclip } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import PhotoListAssociationRow from '@/components/PhotoListAssociationRow.vue';
import PhotoListRow from '@/components/PhotoListRow.vue';
import { useBrowseScroll } from '@/composables/useBrowseScroll';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import {
  assetRowTop,
  buildListRows,
  firstVisibleListRow,
  LIST_GRID_CLASS,
  LIST_ROW_HEIGHT,
  listRowsHeight,
  type ListRow,
} from '@/lib/list-rows';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const metrics = useScrollMetrics(scroller);
const { width, height, scrollTop } = metrics;
useBrowseScroll(scroller, metrics, () => store.browseKey);

const compact = computed(() => width.value > 0 && width.value < 700);
const showFolder = computed(() => store.scanRecursive || store.assets.some((asset) => asset.relativeDir));
const showAssociations = computed(() => store.settings.listShowAssociations);

const rows = computed(() => buildListRows(store.assets, showAssociations.value));
const totalHeight = computed(() => listRowsHeight(rows.value));

const visible = computed<ListRow[]>(() => {
  const all = rows.value;
  if (!all.length) return [];
  const start = firstVisibleListRow(all, scrollTop.value, 400);
  const bottom = scrollTop.value + height.value + 400;
  const slice: ListRow[] = [];
  for (let index = start; index < all.length; index += 1) {
    const row = all[index];
    if (row.top > bottom) break;
    slice.push(row);
  }
  return slice;
});

store.navRowSize = 1;

function rowKey(row: ListRow): string {
  if (row.type === 'asset') return `${row.asset.id}:asset`;
  return `${row.asset.id}:${row.association.kind}:${row.association.path}`;
}

function scrollSelectionIntoView() {
  const el = scroller.value;
  const top = assetRowTop(rows.value, store.selectedIndex);
  if (!el || top === null) return;
  const bottom = top + LIST_ROW_HEIGHT;
  if (top < el.scrollTop) {
    el.scrollTop = top;
  } else if (bottom > el.scrollTop + el.clientHeight) {
    el.scrollTop = bottom - el.clientHeight;
  }
}

watch(() => store.selectedIndex, scrollSelectionIntoView);

watch(
  () => store.browseKey,
  () => {
    if (scroller.value) scroller.value.scrollTop = 0;
  },
);
</script>

<template>
  <div class="flex h-full flex-col">
    <div
      class="grid h-8 shrink-0 items-center gap-3 border-b border-line bg-app/60 px-4 text-2xs font-semibold uppercase tracking-[0.1em] text-subtle"
      :class="compact ? LIST_GRID_CLASS.compact : LIST_GRID_CLASS.full"
    >
      <span>Name</span>
      <span>Rating</span>
      <span>Label</span>
      <span title="Culling flag and favorite">Cull</span>
      <span>Type</span>
      <span>Dimensions</span>
      <template v-if="!compact">
        <span>Size</span>
        <span>Modified</span>
      </template>
      <button
        class="-ml-1.5 flex w-fit items-center gap-1 rounded px-1.5 py-0.5 text-2xs font-semibold uppercase tracking-[0.1em] transition"
        :class="
          showAssociations
            ? 'bg-accent-soft text-accent ring-1 ring-accent/20'
            : 'text-subtle hover:bg-panel-2 hover:text-ink'
        "
        :title="
          showAssociations
            ? 'Hide associated JPG and XMP files'
            : 'Show associated JPG and XMP files beneath each RAW'
        "
        :aria-pressed="showAssociations"
        @click="store.toggleListAssociations()"
      >
        <Paperclip class="h-3 w-3" />
        Assoc.
      </button>
    </div>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
      <div class="relative" :style="{ height: `${totalHeight}px` }">
        <div
          v-for="row in visible"
          :key="rowKey(row)"
          class="absolute inset-x-0"
          :style="{ top: `${row.top}px`, height: `${row.height}px` }"
        >
          <PhotoListRow
            v-if="row.type === 'asset'"
            :asset="row.asset"
            :selected="store.isSelected(row.asset.id)"
            :show-folder="showFolder"
            :compact="compact"
            @select="store.selectWithModifiers(row.asset.id, $event)"
            @open="store.openInFilmstrip(row.asset.id)"
          />
          <PhotoListAssociationRow
            v-else
            :association="row.association"
            :asset-name="row.asset.fileName"
            :compact="compact"
          />
        </div>
      </div>
    </div>
  </div>
</template>
