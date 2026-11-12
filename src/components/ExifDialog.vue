<script setup lang="ts">
import { AlertTriangle, Loader2, Search, X } from 'lucide-vue-next';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { api, errorMessage } from '@/api';
import type { ExifEntry, FullExif, PhotoAsset } from '@/types';

const props = defineProps<{
  asset: PhotoAsset;
}>();

const emit = defineEmits<{
  close: [];
}>();

type LoadStatus = 'loading' | 'ready' | 'error';

const status = ref<LoadStatus>('loading');
const result = ref<FullExif | null>(null);
const error = ref<string | null>(null);
const query = ref('');
const searchInput = ref<HTMLInputElement | null>(null);

/** Reader groups first, maker notes in the middle and derived values last. */
const STANDARD_GROUPS = [
  'ExifIFD',
  'IFD0',
  'GPS',
  'InteropIFD',
  'IFD1',
  'IFD2',
  'IFD3',
  'EXIF',
  'Thumbnail',
];

interface ExifGroup {
  name: string;
  label: string;
  entries: ExifEntry[];
}

function groupLabel(name: string): string {
  if (name === 'ExifIFD') return 'EXIF';
  if (name === 'IFD0') return 'IFD0 · Primary image';
  if (name === 'IFD1') return 'IFD1 · Thumbnail';
  if (name === 'InteropIFD') return 'Interoperability';
  if (name === 'CanonCustom') return 'Canon · Custom functions';
  return name;
}

function groupRank(name: string): number {
  const index = STANDARD_GROUPS.indexOf(name);
  if (index >= 0) return index;
  if (name === 'Composite') return 10_000;
  return 5_000;
}

const total = computed(() => result.value?.entries.length ?? 0);

const filteredGroups = computed<ExifGroup[]>(() => {
  const entries = result.value?.entries ?? [];
  const terms = query.value
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  const groups = new Map<string, ExifEntry[]>();
  for (const entry of entries) {
    if (terms.length) {
      const haystack = `${entry.group} ${entry.tag} ${entry.value}`.toLowerCase();
      if (!terms.every((term) => haystack.includes(term))) continue;
    }
    const list = groups.get(entry.group);
    if (list) list.push(entry);
    else groups.set(entry.group, [entry]);
  }

  return [...groups.entries()]
    .map(([name, groupEntries]) => ({
      name,
      label: groupLabel(name),
      entries: groupEntries,
    }))
    .sort((a, b) => {
      const rank = groupRank(a.name) - groupRank(b.name);
      return rank !== 0 ? rank : a.name.localeCompare(b.name);
    });
});

const matchCount = computed(() =>
  filteredGroups.value.reduce((count, group) => count + group.entries.length, 0),
);

const summary = computed(() => {
  if (status.value !== 'ready') return '';
  if (!query.value.trim()) return `${total.value} tags`;
  return `${matchCount.value} of ${total.value} tags`;
});

const sourceLabel = computed(() => {
  if (!result.value) return '';
  if (result.value.source === 'exiftool') return 'Read with ExifTool';
  if (result.value.source === 'embedded') return 'Read with the built-in EXIF reader';
  return 'No readable metadata';
});

async function load(): Promise<void> {
  status.value = 'loading';
  error.value = null;
  try {
    result.value = await api.fullExif(props.asset);
    status.value = 'ready';
    await nextTick();
    searchInput.value?.focus();
  } catch (caught) {
    error.value = errorMessage(caught);
    status.value = 'error';
  }
}

function clearQuery(): void {
  query.value = '';
  searchInput.value?.focus();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape') return;
  event.preventDefault();
  event.stopPropagation();
  emit('close');
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown, true);
  void load();
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown, true);
});
</script>

<template>
  <AppDialog
    title="Full EXIF"
    :subtitle="asset.fileName"
    full
    @close="emit('close')"
  >
    <div v-if="status === 'loading'" class="flex items-center justify-center py-16">
      <div class="flex items-center gap-2 text-xs text-muted">
        <Loader2 class="h-4 w-4 animate-spin" />
        Reading metadata…
      </div>
    </div>

    <div
      v-else-if="status === 'error'"
      class="flex items-start gap-2 rounded-md border border-danger/25 bg-danger/5 px-3 py-2 text-xs text-danger"
    >
      <AlertTriangle class="mt-0.5 h-3.5 w-3.5 shrink-0" />
      <div class="min-w-0">
        <p class="font-semibold">Metadata could not be read</p>
        <p class="mt-0.5 break-words leading-relaxed text-danger/90">{{ error }}</p>
      </div>
    </div>

    <template v-else>
      <div class="mb-3 flex items-center gap-3">
        <label class="relative min-w-0 flex-1">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-subtle"
          />
          <input
            ref="searchInput"
            v-model="query"
            type="text"
            class="h-8 w-full rounded-md border border-line bg-app px-3 pl-8 pr-8 text-xs text-ink outline-none transition placeholder:text-subtle focus:border-accent focus:ring-2 focus:ring-accent/25"
            placeholder="Filter by tag, group or value…"
            autocomplete="off"
            spellcheck="false"
            aria-label="Filter EXIF tags"
          />
          <button
            v-if="query"
            type="button"
            class="absolute right-1.5 top-1/2 flex h-5 w-5 -translate-y-1/2 items-center justify-center rounded text-subtle transition hover:bg-panel-2 hover:text-ink"
            title="Clear filter"
            aria-label="Clear filter"
            @click="clearQuery"
          >
            <X class="h-3 w-3" />
          </button>
        </label>
        <span class="shrink-0 text-2xs tabular-nums text-subtle">{{ summary }}</span>
      </div>

      <div
        v-if="!result?.entries.length"
        class="rounded-md border border-dashed border-line px-3 py-8 text-center text-xs text-subtle"
      >
        No EXIF metadata was found for this photo.
      </div>

      <div
        v-else-if="!filteredGroups.length"
        class="rounded-md border border-dashed border-line px-3 py-8 text-center text-xs text-subtle"
      >
        No tags match “{{ query.trim() }}”.
      </div>

      <div v-else class="space-y-4">
        <section
          v-for="group in filteredGroups"
          :key="group.name"
          class="overflow-hidden rounded-md border border-line bg-app/40"
        >
          <header
            class="flex items-center justify-between gap-3 border-b border-line bg-panel-2/70 px-3 py-1.5"
          >
            <h3 class="panel-heading">{{ group.label }}</h3>
            <span class="text-3xs tabular-nums text-subtle">
              {{ group.entries.length }}
            </span>
          </header>
          <dl class="divide-y divide-line/60">
            <div
              v-for="(entry, index) in group.entries"
              :key="`${entry.group}:${entry.tag}:${index}`"
              class="grid grid-cols-[minmax(150px,220px)_minmax(0,1fr)] gap-4 px-3 py-1.5 transition hover:bg-panel-2/40"
            >
              <dt class="break-words font-mono text-2xs text-subtle">{{ entry.tag }}</dt>
              <dd class="min-w-0 break-words font-mono text-2xs text-ink selectable">
                {{ entry.value }}
              </dd>
            </div>
          </dl>
        </section>
      </div>
    </template>

    <template #footer>
      <span class="mr-auto text-2xs text-subtle">{{ sourceLabel }}</span>
      <button class="btn" @click="emit('close')">Close</button>
    </template>
  </AppDialog>
</template>
