<script setup lang="ts">
import {
  AlertTriangle,
  ChartNoAxesColumn,
  Check,
  ExternalLink,
  FileText,
  FolderOpen,
  Image as ImageIcon,
  Info,
  Loader2,
  Maximize2,
  PanelRightClose,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import AssociationBadges from '@/components/AssociationBadges.vue';
import MetadataEditor from '@/components/MetadataEditor.vue';
import PhotoHistogram from '@/components/PhotoHistogram.vue';
import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import ResizeHandle from '@/components/ResizeHandle.vue';
import SourceBadge from '@/components/SourceBadge.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { api, errorMessage } from '@/api';
import {
  basename,
  formatBytes,
  formatCount,
  formatDate,
  formatDimensions,
  formatDuration,
  kindLabel,
  sourceDescription,
} from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { HistogramData } from '@/types';

const store = useLibraryStore();

const asset = computed(() => store.selectedAsset);

const thumb = useThumbnail(
  () => store.selectedAsset,
  () => Math.max(240, store.settings.infoWidth - 24),
);

type DetailTab = 'preview' | 'histogram';

const detailTab = ref<DetailTab>('preview');
const histogram = ref<HistogramData | null>(null);
const histogramStatus = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
const histogramError = ref<string | null>(null);
const histogramCache = new Map<string, HistogramData>();
let histogramRequest = 0;

function resetHistogram(): void {
  histogramRequest += 1;
  histogram.value = null;
  histogramStatus.value = 'idle';
  histogramError.value = null;
}

function rememberHistogram(key: string, data: HistogramData): void {
  if (histogramCache.size >= 64) {
    const oldest = histogramCache.keys().next().value;
    if (oldest) histogramCache.delete(oldest);
  }
  histogramCache.set(key, data);
}

async function loadHistogram(): Promise<void> {
  const current = asset.value;
  if (!current || detailTab.value !== 'histogram') {
    resetHistogram();
    return;
  }

  const preview = thumb.value;
  if (!preview || preview.status === 'loading') {
    resetHistogram();
    histogramStatus.value = 'loading';
    return;
  }

  const key = `${current.id}:${preview.edge}`;
  const cached = histogramCache.get(key);
  if (cached) {
    resetHistogram();
    histogram.value = cached;
    histogramStatus.value = 'ready';
    return;
  }

  const request = ++histogramRequest;
  histogram.value = null;
  histogramError.value = null;
  histogramStatus.value = 'loading';
  try {
    const data = await api.histogram(current, preview.edge);
    if (request !== histogramRequest) return;
    rememberHistogram(key, data);
    histogram.value = data;
    histogramStatus.value = 'ready';
  } catch (error) {
    if (request !== histogramRequest) return;
    histogramError.value = errorMessage(error);
    histogramStatus.value = 'error';
  }
}

watch(
  [
    () => asset.value?.id ?? null,
    () => detailTab.value,
    () => thumb.value?.status ?? 'idle',
    () => thumb.value?.edge ?? 0,
  ],
  () => {
    void loadHistogram();
  },
  { immediate: true },
);

const dimensions = computed(() => {
  if (!asset.value) return '—';
  const known = dimensionsFor(asset.value);
  return known ? formatDimensions(known.width, known.height) : '—';
});

const source = computed(() => thumb.value?.source ?? asset.value?.previewHint ?? 'placeholder');

function resize(width: number) {
  store.settings.infoWidth = width;
}

function commitResize(width: number) {
  store.updateSettings({ infoWidth: width });
}

async function openPrimary() {
  if (!asset.value) return;
  try {
    await api.openAsset(asset.value.primaryPath);
  } catch (error) {
    store.addToast(errorMessage(error), 'error');
  }
}

async function revealPrimary() {
  if (!asset.value) return;
  try {
    await api.revealAsset(asset.value.primaryPath);
  } catch (error) {
    store.addToast(errorMessage(error), 'error');
  }
}
</script>

<template>
  <aside
    class="relative flex h-full shrink-0 flex-col border-l border-line bg-panel"
    :style="{ width: `${store.settings.infoWidth}px` }"
  >
    <div class="flex h-10 shrink-0 items-center justify-between px-3">
      <span class="panel-heading">Info</span>
      <button
        class="btn-icon-sm"
        :title="`Hide info panel (${store.shortcutLabel('library.info')})`"
        @click="store.toggleInfo()"
      >
        <PanelRightClose class="h-3.5 w-3.5" />
      </button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto">
      <template v-if="asset">
        <div class="px-3">
          <div class="mb-1.5 flex items-center justify-between gap-2">
            <div
              class="inline-flex items-center gap-0.5 rounded-md border border-line bg-app/60 p-0.5"
              role="tablist"
              aria-label="Photo preview and histogram"
            >
              <button
                role="tab"
                class="flex items-center gap-1 rounded px-2 py-1 text-2xs font-semibold transition"
                :class="
                  detailTab === 'preview'
                    ? 'bg-accent-soft text-accent'
                    : 'text-subtle hover:bg-panel-2/60 hover:text-ink'
                "
                :aria-selected="detailTab === 'preview'"
                @click="detailTab = 'preview'"
              >
                <ImageIcon class="h-3 w-3" />
                Preview
              </button>
              <button
                role="tab"
                class="flex items-center gap-1 rounded px-2 py-1 text-2xs font-semibold transition"
                :class="
                  detailTab === 'histogram'
                    ? 'bg-accent-soft text-accent'
                    : 'text-subtle hover:bg-panel-2/60 hover:text-ink'
                "
                :aria-selected="detailTab === 'histogram'"
                @click="detailTab = 'histogram'"
              >
                <ChartNoAxesColumn class="h-3 w-3" />
                Histogram
              </button>
            </div>
            <span
              v-if="detailTab === 'histogram' && histogram"
              class="text-2xs tabular-nums text-subtle"
            >
              {{ formatCount(histogram.pixels) }} px
            </span>
          </div>

          <div
            class="relative h-44 overflow-hidden rounded-md bg-canvas ring-1 ring-line"
            role="tabpanel"
          >
            <div v-if="detailTab === 'preview'" class="relative flex h-full items-center justify-center">
              <PhotoPlaceholder
                v-if="!thumb?.src"
                :asset="asset"
                :loading="!thumb || thumb.status === 'loading'"
                :error="thumb?.error"
              />
              <img
                v-else
                :src="thumb.src"
                :alt="asset.fileName"
                class="max-h-full max-w-full object-contain"
                draggable="false"
                @dblclick="store.openFullscreen()"
                @error="markPreviewFailed(thumb)"
              />
              <div class="pointer-events-none absolute inset-x-0 top-0 flex bg-gradient-to-b from-black/50 to-transparent p-2">
                <SourceBadge :source="source" overlay />
              </div>
              <Loader2
                v-if="thumb?.status === 'loading'"
                class="absolute bottom-2 right-2 h-3 w-3 animate-spin text-subtle"
              />
            </div>
            <PhotoHistogram
              v-else
              class="h-full"
              :data="histogram"
              :loading="histogramStatus === 'loading'"
              :error="histogramError"
            />
          </div>

          <h3 class="mt-3 break-all text-xs font-semibold leading-snug text-ink">
            {{ asset.fileName }}
          </h3>
          <p class="mt-1 break-all text-2xs leading-snug text-subtle selectable">
            {{ asset.primaryPath }}
          </p>

          <div class="mt-2.5 flex gap-1.5">
            <button class="btn flex-1" title="Open with the default application" @click="openPrimary()">
              <ExternalLink class="h-3.5 w-3.5" />
              Open
            </button>
            <button class="btn flex-1" title="Show the file in Finder" @click="revealPrimary()">
              <FolderOpen class="h-3.5 w-3.5" />
              Reveal
            </button>
          </div>
          <button
            class="btn mt-1.5 w-full"
            :title="`Open in fullscreen viewer (${store.shortcutLabel('browse.fullscreen')})`"
            @click="store.openFullscreen()"
          >
            <Maximize2 class="h-3.5 w-3.5" />
            Fullscreen
          </button>
          <button
            class="btn mt-1.5 w-full text-danger hover:border-danger/40 hover:bg-danger/10"
            :title="
              store.selectedCount > 1
                ? `Delete ${store.selectedCount} selected photos`
                : 'Delete this photo'
            "
            @click="store.openDeleteAssets()"
          >
            <Trash2 class="h-3.5 w-3.5" />
            {{ store.selectedCount > 1 ? `Delete ${store.selectedCount} photos…` : 'Delete photo…' }}
          </button>
        </div>

        <MetadataEditor />

        <section class="border-t border-line px-3 py-3">
          <h4 class="panel-heading mb-1.5">File</h4>
          <div class="info-row">
            <span class="info-label">Type</span>
            <span class="info-value">
              {{ kindLabel(asset.kind, asset.extension) }}
              <span class="text-subtle">· .{{ asset.extension }}</span>
            </span>
          </div>
          <div class="info-row">
            <span class="info-label">Size</span>
            <span class="info-value tabular-nums">{{ formatBytes(asset.fileSize) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Modified</span>
            <span class="info-value">{{ formatDate(asset.modifiedMs) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Dimensions</span>
            <span class="info-value tabular-nums">{{ dimensions }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Folder</span>
            <span class="info-value truncate" :title="asset.relativeDir || '/'">
              {{ asset.relativeDir || 'Library root' }}
            </span>
          </div>
        </section>

        <section class="border-t border-line px-3 py-3">
          <h4 class="panel-heading mb-1.5">Preview source</h4>
          <div class="flex items-center gap-2">
            <SourceBadge :source="source" />
            <span v-if="thumb?.edge" class="text-2xs tabular-nums text-subtle">
              {{ thumb.edge }} px cache
            </span>
          </div>
          <p class="mt-1.5 text-2xs leading-relaxed text-muted">
            {{ sourceDescription(source) }}
          </p>
          <p v-if="thumb?.error" class="mt-1.5 text-2xs leading-relaxed text-danger">
            {{ thumb.error }}
          </p>
        </section>

        <section class="border-t border-line px-3 py-3">
          <div class="mb-1.5 flex min-h-4 items-center justify-between gap-2">
            <h4 class="panel-heading">Associations</h4>
            <span
              v-if="asset.companionJpeg || asset.xmpSidecar"
              class="flex shrink-0 items-center gap-1"
            >
              <AssociationBadges :asset="asset" />
            </span>
          </div>

          <div class="info-row">
            <span class="info-label">Companion JPEG</span>
            <span
              class="info-value flex items-center justify-end gap-1"
              :class="asset.companionJpeg ? 'text-ink' : 'text-subtle'"
            >
              <Check v-if="asset.companionJpeg" class="h-3 w-3 shrink-0 text-success" />
              <span class="truncate" :title="asset.companionJpeg ?? ''">
                {{ asset.companionJpeg ? basename(asset.companionJpeg) : 'Not found' }}
              </span>
            </span>
          </div>

          <div class="info-row">
            <span class="info-label">XMP sidecar</span>
            <span
              class="info-value flex items-center justify-end gap-1"
              :class="asset.xmpSidecar ? 'text-ink' : 'text-subtle'"
            >
              <FileText v-if="asset.xmpSidecar" class="h-3 w-3 shrink-0 text-success" />
              <span class="truncate" :title="asset.xmpSidecar ?? ''">
                {{ asset.xmpSidecar ? basename(asset.xmpSidecar) : 'Not found' }}
              </span>
            </span>
          </div>

          <template v-if="asset.extraCompanions.length">
            <div class="info-row">
              <span class="info-label">Other JPEGs</span>
              <span class="info-value text-subtle">
                <span v-for="path in asset.extraCompanions" :key="path" class="block truncate" :title="path">
                  {{ basename(path) }}
                </span>
              </span>
            </div>
          </template>

          <template v-if="asset.extraSidecars.length">
            <div class="info-row">
              <span class="info-label">Other XMP</span>
              <span class="info-value text-subtle">
                <span v-for="path in asset.extraSidecars" :key="path" class="block truncate" :title="path">
                  {{ basename(path) }}
                </span>
              </span>
            </div>
          </template>
        </section>
      </template>

      <template v-else-if="store.selectedFolder">
        <div class="px-3 py-2">
          <div class="flex items-center gap-2">
            <FolderOpen class="h-3.5 w-3.5 shrink-0 text-subtle" />
            <h3 class="min-w-0 truncate text-xs font-semibold text-ink" :title="store.selectedFolder">
              {{ store.folderName }}
            </h3>
          </div>
          <p class="mt-1 break-all text-2xs leading-snug text-subtle selectable">
            {{ store.relativeFolderLabel }}
          </p>
        </div>

        <section class="mt-2 border-t border-line px-3 py-3">
          <h4 class="panel-heading mb-2">Contents</h4>
          <div class="grid grid-cols-2 gap-1.5">
            <div class="rounded-md border border-line bg-elevated px-2.5 py-2">
              <p class="text-base font-semibold tabular-nums leading-tight text-ink">
                {{ formatCount(store.stats.total) }}
              </p>
              <p class="text-2xs text-subtle">Photos</p>
            </div>
            <div class="rounded-md border border-line bg-elevated px-2.5 py-2">
              <p class="text-base font-semibold tabular-nums leading-tight text-ink">
                {{ formatCount(store.stats.raw) }}
              </p>
              <p class="text-2xs text-subtle">RAW files</p>
            </div>
            <div class="rounded-md border border-line bg-elevated px-2.5 py-2">
              <p class="text-base font-semibold tabular-nums leading-tight text-ink">
                {{ formatCount(store.stats.jpeg) }}
              </p>
              <p class="text-2xs text-subtle">JPEG files</p>
            </div>
            <div class="rounded-md border border-line bg-elevated px-2.5 py-2">
              <p class="text-base font-semibold tabular-nums leading-tight text-ink">
                {{ formatCount(store.stats.other) }}
              </p>
              <p class="text-2xs text-subtle">Other images</p>
            </div>
          </div>
        </section>

        <section class="border-t border-line px-3 py-3">
          <h4 class="panel-heading mb-1.5">Associations</h4>
          <div class="info-row">
            <span class="info-label">With XMP sidecar</span>
            <span class="info-value tabular-nums">{{ formatCount(store.stats.withXmp) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">RAW + JPEG pairs</span>
            <span class="info-value tabular-nums">{{ formatCount(store.stats.withCompanion) }}</span>
          </div>
        </section>

        <section class="border-t border-line px-3 py-3">
          <h4 class="panel-heading mb-1.5">Scan</h4>
          <div class="info-row">
            <span class="info-label">Subfolders</span>
            <span class="info-value">{{ store.settings.recursive ? 'Included' : 'Not included' }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Duration</span>
            <span class="info-value tabular-nums">{{ formatDuration(store.scanElapsedMs) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Warnings</span>
            <button
              v-if="store.warningsCount"
              class="info-value flex items-center gap-1 text-warning hover:underline"
              @click="store.openWarnings()"
            >
              <AlertTriangle class="h-3 w-3" />
              {{ store.warningsCount }}
            </button>
            <span v-else class="info-value text-subtle">None</span>
          </div>
        </section>

        <p class="mx-3 my-3 rounded-md border border-dashed border-line px-2.5 py-2 text-2xs leading-relaxed text-subtle">
          Select a photo to inspect its files and preview source.
        </p>
      </template>

      <div v-else class="flex h-full flex-col items-center justify-center px-6 text-center text-subtle">
        <Info class="mb-2 h-6 w-6 opacity-60" />
        <p class="text-xs text-muted">Nothing selected</p>
        <p class="mt-1 text-2xs">Choose a folder and photo to see details here.</p>
      </div>
    </div>

    <ResizeHandle
      side="left"
      :width="store.settings.infoWidth"
      :min="260"
      :max="520"
      @resize="resize"
      @commit="commitResize"
    />
  </aside>
</template>
