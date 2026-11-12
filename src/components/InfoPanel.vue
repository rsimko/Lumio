<script setup lang="ts">
import {
  AlertTriangle,
  ChartNoAxesColumn,
  ExternalLink,
  FileSearch,
  FolderOpen,
  Image as ImageIcon,
  Images,
  Info,
  Loader2,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import ExifDialog from '@/components/ExifDialog.vue';
import InfoSection from '@/components/InfoSection.vue';
import IptcEditor from '@/components/IptcEditor.vue';
import KeywordsEditor from '@/components/KeywordsEditor.vue';
import LocationEditor from '@/components/LocationEditor.vue';
import MetadataEditor from '@/components/MetadataEditor.vue';
import PhotoHistogram from '@/components/PhotoHistogram.vue';
import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import ResizeHandle from '@/components/ResizeHandle.vue';
import SourceBadge from '@/components/SourceBadge.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { api, errorMessage } from '@/api';
import { assetPreviewKey } from '@/lib/assets';
import {
  basename,
  formatAperture,
  formatBytes,
  formatCount,
  formatDate,
  formatDimensions,
  formatDuration,
  formatFocalLength,
  formatShutterSpeed,
  kindLabel,
} from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { CameraInfo, HistogramData, PhotoAsset } from '@/types';

const store = useLibraryStore();

const asset = computed(() => store.selectedAsset);

const thumb = useThumbnail(
  () => store.selectedAsset,
  () => Math.max(240, store.settings.infoWidth - 24),
);

type DetailTab = 'preview' | 'histogram';

const detailTab = ref<DetailTab>('histogram');
type HistogramMode = 'preview' | 'raw';
const histogramMode = ref<HistogramMode>('preview');
const histogram = ref<HistogramData | null>(null);
const histogramStatus = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
const histogramError = ref<string | null>(null);
const histogramCache = new Map<string, HistogramData>();
let histogramRequest = 0;

const showRawHistogram = computed(
  () => histogramMode.value === 'raw' && asset.value?.kind === 'raw',
);

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

  if (showRawHistogram.value) {
    const rawKey = `${current.id}:raw`;
    const cached = histogramCache.get(rawKey);
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
      const data = await api.rawHistogram(current);
      if (request !== histogramRequest) return;
      rememberHistogram(rawKey, data);
      histogram.value = data;
      histogramStatus.value = 'ready';
    } catch (error) {
      if (request !== histogramRequest) return;
      histogramError.value = errorMessage(error);
      histogramStatus.value = 'error';
    }
    return;
  }

  const preview = thumb.value;
  if (!preview || preview.status === 'loading') {
    resetHistogram();
    histogramStatus.value = 'loading';
    return;
  }

  const key = `${assetPreviewKey(current)}:${preview.edge}`;
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
    () => showRawHistogram.value,
    () => thumb.value?.status ?? 'idle',
    () => thumb.value?.edge ?? 0,
  ],
  () => {
    void loadHistogram();
  },
  { immediate: true },
);

function toggleRawHistogram(): void {
  histogramMode.value = histogramMode.value === 'raw' ? 'preview' : 'raw';
}

const cameraInfo = ref<CameraInfo | null>(null);
const cameraStatus = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
const cameraError = ref<string | null>(null);
const cameraCache = new Map<string, CameraInfo>();
let cameraRequest = 0;

function rememberCameraInfo(key: string, data: CameraInfo): void {
  if (cameraCache.size >= 128) {
    const oldest = cameraCache.keys().next().value;
    if (oldest) cameraCache.delete(oldest);
  }
  cameraCache.set(key, data);
}

async function loadCameraInfo(): Promise<void> {
  const current = asset.value;
  const request = ++cameraRequest;
  if (!current) {
    cameraInfo.value = null;
    cameraStatus.value = 'idle';
    cameraError.value = null;
    return;
  }

  const key = `${current.id}:${current.companionJpeg ?? ''}`;
  const cached = cameraCache.get(key);
  if (cached) {
    cameraInfo.value = cached;
    cameraStatus.value = 'ready';
    cameraError.value = null;
    return;
  }

  cameraInfo.value = null;
  cameraStatus.value = 'loading';
  cameraError.value = null;
  try {
    const data = await api.cameraInfo(current);
    if (request !== cameraRequest) return;
    rememberCameraInfo(key, data);
    cameraInfo.value = data;
    cameraStatus.value = 'ready';
  } catch (error) {
    if (request !== cameraRequest) return;
    cameraError.value = errorMessage(error);
    cameraStatus.value = 'error';
  }
}

watch(
  [() => asset.value?.id ?? null, () => asset.value?.companionJpeg ?? null],
  () => {
    void loadCameraInfo();
  },
  { immediate: true },
);

const hasCameraInfo = computed(() => {
  const info = cameraInfo.value;
  return Boolean(
    info &&
      (info.make ||
        info.model ||
        info.lens ||
        info.shutterSpeed != null ||
        info.aperture != null ||
        info.iso != null ||
        info.focalLength != null ||
        info.latitude != null ||
        info.longitude != null ||
        info.altitude != null),
  );
});

const exifOpen = ref(false);
const exifAsset = ref<PhotoAsset | null>(null);

function openExif(): void {
  const current = asset.value;
  if (!current) return;
  exifAsset.value = current;
  exifOpen.value = true;
}

function closeExif(): void {
  exifOpen.value = false;
  exifAsset.value = null;
}

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

async function revealAssociation(path: string | null): Promise<void> {
  if (!path) return;
  try {
    await api.revealAsset(path);
  } catch (error) {
    store.addToast(errorMessage(error), 'error');
  }
}

function associationTitle(kind: 'jpeg' | 'xmp'): string {
  const current = asset.value;
  if (!current) return '';
  const path = kind === 'jpeg' ? current.companionJpeg : current.xmpSidecar;
  const label = kind === 'jpeg' ? 'companion JPEG' : 'XMP sidecar';
  if (!path) return `No ${label} found`;
  const extra =
    kind === 'jpeg' ? current.extraCompanions.length : current.extraSidecars.length;
  return `Reveal ${basename(path)} in Finder${extra ? ` (+${extra} more)` : ''}`;
}

function revealAssociationKind(kind: 'jpeg' | 'xmp'): void {
  const current = asset.value;
  if (!current) return;
  void revealAssociation(kind === 'jpeg' ? current.companionJpeg : current.xmpSidecar);
}
</script>

<template>
  <aside
    class="relative flex h-full shrink-0 flex-col border-l border-line bg-panel"
    :style="{ width: `${store.settings.infoWidth}px` }"
  >
    <div class="flex h-10 shrink-0 items-center px-3">
      <span class="panel-heading">Info</span>
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
            <button
              v-if="detailTab === 'histogram' && asset.kind === 'raw'"
              class="rounded border px-1.5 py-0.5 text-3xs font-semibold uppercase tracking-wider transition"
              :class="
                showRawHistogram
                  ? 'border-accent/40 bg-accent-soft text-accent'
                  : 'border-line bg-panel text-subtle hover:bg-panel-2 hover:text-ink'
              "
              :aria-pressed="showRawHistogram"
              :title="
                showRawHistogram
                  ? 'Show the histogram from the rendered preview'
                  : 'Show the histogram from RAW sensor data'
              "
              @click="toggleRawHistogram"
            >
              RAW
            </button>
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
                <SourceBadge :source="source" :kind="asset.kind" overlay />
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
              :raw="showRawHistogram"
            />
          </div>

          <div class="mb-2 mt-2.5 flex items-center gap-1.5">
            <button
              class="btn min-w-0 flex-1 justify-center px-2"
              title="Open with the default application"
              @click="openPrimary()"
            >
              <ExternalLink class="h-3.5 w-3.5 shrink-0" />
              <span class="truncate">Open</span>
            </button>
            <button
              class="btn min-w-0 flex-1 justify-center px-2"
              title="Show the file in Finder"
              @click="revealPrimary()"
            >
              <FolderOpen class="h-3.5 w-3.5 shrink-0" />
              <span class="truncate">Reveal</span>
            </button>
            <button
              class="btn min-w-0 flex-1 justify-center px-2 text-danger hover:border-danger/40 hover:bg-danger/10"
              :title="
                store.selectedCount > 1
                  ? `Delete ${store.selectedCount} selected photos`
                  : 'Delete this photo'
              "
              @click="store.openDeleteAssets()"
            >
              <Trash2 class="h-3.5 w-3.5 shrink-0" />
              <span class="truncate">Delete</span>
            </button>
          </div>
        </div>

        <MetadataEditor class="mb-2" />

        <InfoSection title="File" class="info-section-left">
          <div class="info-row">
            <span class="info-label">Filename</span>
            <span class="info-value break-all selectable" :title="asset.fileName">
              {{ asset.fileName }}
            </span>
          </div>
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
          <div class="info-row">
            <span class="info-label">Associations</span>
            <span class="flex flex-wrap items-center gap-1">
              <button
                type="button"
                class="badge rounded-full ring-1 transition"
                :class="
                  asset.companionJpeg
                    ? 'cursor-pointer bg-success/15 text-success ring-success/20 hover:bg-success/25'
                    : 'cursor-default bg-panel-2/60 text-subtle/70 ring-line'
                "
                :title="associationTitle('jpeg')"
                :aria-label="associationTitle('jpeg')"
                :aria-disabled="!asset.companionJpeg"
                @click="revealAssociationKind('jpeg')"
              >
                JPG
              </button>
              <button
                type="button"
                class="badge rounded-full ring-1 transition"
                :class="
                  asset.xmpSidecar
                    ? 'cursor-pointer bg-accent-soft text-accent ring-accent/20 hover:bg-accent/20'
                    : 'cursor-default bg-panel-2/60 text-subtle/70 ring-line'
                "
                :title="associationTitle('xmp')"
                :aria-label="associationTitle('xmp')"
                :aria-disabled="!asset.xmpSidecar"
                @click="revealAssociationKind('xmp')"
              >
                XMP
              </button>
            </span>
          </div>
        </InfoSection>

        <InfoSection title="Camera and lens" class="info-section-left">
          <template #actions>
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-3xs font-semibold uppercase tracking-wider text-subtle transition hover:border-line-strong hover:bg-panel-2 hover:text-ink"
              title="View every EXIF tag for this photo"
              @click="openExif"
            >
              <FileSearch class="h-3 w-3" />
              Full EXIF
            </button>
          </template>
          <div v-if="cameraStatus === 'loading'" class="py-0.5 text-2xs text-subtle">
            Reading camera metadata…
          </div>
          <div
            v-else-if="cameraStatus === 'error'"
            class="py-0.5 text-2xs leading-relaxed text-danger"
          >
            {{ cameraError }}
          </div>
          <template v-else-if="hasCameraInfo">
            <div v-if="cameraInfo?.make" class="info-row">
              <span class="info-label">Camera make</span>
              <span class="info-value">{{ cameraInfo.make }}</span>
            </div>
            <div v-if="cameraInfo?.model" class="info-row">
              <span class="info-label">Camera model</span>
              <span class="info-value">{{ cameraInfo.model }}</span>
            </div>
            <div v-if="cameraInfo?.lens" class="info-row">
              <span class="info-label">Lens</span>
              <span class="info-value">{{ cameraInfo.lens }}</span>
            </div>
            <div v-if="cameraInfo?.shutterSpeed != null" class="info-row">
              <span class="info-label">Shutter</span>
              <span class="info-value tabular-nums">
                {{ formatShutterSpeed(cameraInfo.shutterSpeed) }}
              </span>
            </div>
            <div v-if="cameraInfo?.aperture != null" class="info-row">
              <span class="info-label">Aperture</span>
              <span class="info-value tabular-nums">
                {{ formatAperture(cameraInfo.aperture) }}
              </span>
            </div>
            <div v-if="cameraInfo?.iso != null" class="info-row">
              <span class="info-label">ISO</span>
              <span class="info-value tabular-nums">{{ cameraInfo.iso }}</span>
            </div>
            <div v-if="cameraInfo?.focalLength != null" class="info-row">
              <span class="info-label">Focal length</span>
              <span class="info-value tabular-nums">
                {{ formatFocalLength(cameraInfo.focalLength) }}
              </span>
            </div>
          </template>
          <p v-else class="py-0.5 text-2xs text-subtle">No camera metadata found.</p>
        </InfoSection>

        <IptcEditor class="mb-2" />
        <LocationEditor
          class="mb-2"
          :camera-latitude="cameraInfo?.latitude ?? null"
          :camera-longitude="cameraInfo?.longitude ?? null"
          :camera-altitude="cameraInfo?.altitude ?? null"
        />
        <KeywordsEditor class="mb-2" />
      </template>

      <template v-else-if="store.selectedFolder || store.isLibraryScope">
        <div class="px-3 py-2">
          <div class="flex items-center gap-2">
            <FolderOpen v-if="!store.isLibraryScope" class="h-3.5 w-3.5 shrink-0 text-subtle" />
            <Images v-else class="h-3.5 w-3.5 shrink-0 text-subtle" />
            <h3
              class="min-w-0 truncate text-xs font-semibold text-ink"
              :title="store.isLibraryScope ? store.viewSubtitle : (store.selectedFolder ?? '')"
            >
              {{ store.viewTitle }}
            </h3>
          </div>
          <p class="mt-1 break-all text-2xs leading-snug text-subtle selectable">
            {{ store.viewSubtitle }}
          </p>
        </div>

        <InfoSection :title="store.isLibraryScope ? 'Library contents' : 'Contents'">
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
        </InfoSection>

        <InfoSection title="Associations">
          <div class="info-row">
            <span class="info-label">With XMP sidecar</span>
            <span class="info-value tabular-nums">{{ formatCount(store.stats.withXmp) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">RAW + JPEG pairs</span>
            <span class="info-value tabular-nums">{{ formatCount(store.stats.withCompanion) }}</span>
          </div>
        </InfoSection>

        <InfoSection title="Scan">
          <div class="info-row">
            <span class="info-label">{{ store.isLibraryScope ? 'Scope' : 'Subfolders' }}</span>
            <span class="info-value">
              {{
                store.isLibraryScope
                  ? 'Entire library'
                  : store.settings.recursive
                    ? 'Included'
                    : 'Not included'
              }}
            </span>
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
        </InfoSection>

        <p class="mx-3 my-3 rounded-md border border-dashed border-line px-2.5 py-2 text-2xs leading-relaxed text-subtle">
          Select a photo to inspect its files and metadata.
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

  <ExifDialog v-if="exifOpen && exifAsset" :asset="exifAsset" @close="closeExif" />
</template>
