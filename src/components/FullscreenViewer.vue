<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ChevronLeft, ChevronRight, X } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { formatCount, formatDimensions, sourceLabel } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const viewport = ref({
  width: typeof window === 'undefined' ? 1440 : window.innerWidth,
  height: typeof window === 'undefined' ? 900 : window.innerHeight,
});
const controlsVisible = ref(true);
let hideTimer: number | undefined;

const thumb = useThumbnail(
  () => store.selectedAsset,
  () => Math.max(viewport.value.width, viewport.value.height),
);

const sourceText = computed(() => {
  if (!thumb.value || thumb.value.status === 'loading') return 'Loading preview…';
  if (thumb.value.status === 'error') return 'Preview unavailable';
  return sourceLabel(thumb.value.source);
});

const sourceDot = computed(() => {
  if (!thumb.value || thumb.value.status === 'loading') return 'bg-white/50 animate-pulse';
  if (thumb.value.status === 'error') return 'bg-danger';
  switch (thumb.value.source) {
    case 'companionJpeg':
      return 'bg-success';
    case 'embeddedRaw':
      return 'bg-warning';
    case 'original':
      return 'bg-accent';
    default:
      return 'bg-danger';
  }
});

const dimensions = computed(() => {
  const asset = store.selectedAsset;
  if (!asset) return '—';
  const known = dimensionsFor(asset);
  return known ? formatDimensions(known.width, known.height) : '—';
});

function wake() {
  controlsVisible.value = true;
  if (hideTimer) window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    controlsVisible.value = false;
  }, 2400);
}

function onResize() {
  viewport.value = { width: window.innerWidth, height: window.innerHeight };
}

function close() {
  store.closeFullscreen();
}

function previous() {
  store.navigateHorizontal(-1);
  wake();
}

function next() {
  store.navigateHorizontal(1);
  wake();
}

async function focusWindow(): Promise<void> {
  try {
    await getCurrentWindow().setFocus();
  } catch (error) {
    console.error('Could not focus the Lumio window', error);
  }
  window.focus();
}

async function enterFullscreen(): Promise<void> {
  try {
    await getCurrentWindow().setFullscreen(true);
  } catch (error) {
    console.error('Could not enter native fullscreen', error);
  }
  // Native fullscreen can leave the webview without a first responder.
  await focusWindow();
}

async function leaveFullscreen(): Promise<void> {
  try {
    await getCurrentWindow().setFullscreen(false);
  } catch (error) {
    console.error('Could not leave native fullscreen', error);
  }
  // Focus may only settle once macOS has finished the exit animation, so
  // restore it immediately and once more after the transition.
  await focusWindow();
  window.setTimeout(() => {
    void focusWindow();
  }, 320);
}

onMounted(() => {
  void enterFullscreen();
  window.addEventListener('resize', onResize);
  window.addEventListener('mousemove', wake);
  wake();
});

onBeforeUnmount(() => {
  if (hideTimer) window.clearTimeout(hideTimer);
  window.removeEventListener('resize', onResize);
  window.removeEventListener('mousemove', wake);
  void leaveFullscreen();
});
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex flex-col bg-black"
    :class="controlsVisible ? '' : 'cursor-none-force'"
    role="dialog"
    aria-modal="true"
    aria-label="Fullscreen photo viewer"
    @mousemove="wake"
  >
    <div class="relative flex h-full w-full items-center justify-center overflow-hidden">
      <img
        v-if="thumb?.src"
        :src="thumb.src"
        :alt="store.selectedAsset?.fileName ?? ''"
        class="block h-auto max-h-full w-auto max-w-full select-none object-contain"
        draggable="false"
        @error="markPreviewFailed(thumb)"
      />
      <div
        v-else-if="store.selectedAsset"
        class="flex h-64 w-96 max-w-[80vw] flex-col overflow-hidden rounded-lg border border-white/10 bg-white/5"
      >
        <PhotoPlaceholder
          :asset="store.selectedAsset"
          :loading="!thumb || thumb.status === 'loading'"
          :error="thumb?.error"
        />
      </div>

      <div
        class="pointer-events-none absolute inset-x-0 top-0 flex items-start justify-between gap-4 bg-gradient-to-b from-black/75 via-black/35 to-transparent px-5 pb-10 pt-4 transition-opacity duration-200"
        :class="controlsVisible ? 'opacity-100' : 'opacity-0'"
      >
        <div class="min-w-0">
          <p class="truncate text-sm font-medium text-white/95">
            {{ store.selectedAsset?.fileName }}
          </p>
          <div class="mt-1 flex flex-wrap items-center gap-2 text-2xs text-white/65">
            <span class="badge bg-white/10 text-white/85 ring-1 ring-white/15">
              <span class="h-1.5 w-1.5 rounded-full" :class="sourceDot" />
              {{ sourceText }}
            </span>
            <span class="tabular-nums">{{ dimensions }}</span>
            <span v-if="store.selectedAsset?.xmpSidecar" class="flex items-center gap-1">
              XMP sidecar
            </span>
            <span v-if="store.selectedAsset?.companionJpeg" class="flex items-center gap-1">
              Companion JPEG
            </span>
          </div>
        </div>
        <button
          class="pointer-events-auto grid h-8 w-8 shrink-0 place-items-center rounded-full bg-white/10 text-white/85 transition hover:bg-white/20"
          title="Exit fullscreen (Esc)"
          @click="close"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <button
        class="absolute left-4 top-1/2 grid h-10 w-10 -translate-y-1/2 place-items-center rounded-full bg-white/10 text-white/85 backdrop-blur transition hover:bg-white/20 disabled:pointer-events-none disabled:opacity-25"
        :class="controlsVisible ? 'opacity-100' : 'pointer-events-none opacity-0'"
        title="Previous photo (←)"
        :disabled="store.selectedIndex <= 0"
        @click="previous"
      >
        <ChevronLeft class="h-5 w-5" />
      </button>
      <button
        class="absolute right-4 top-1/2 grid h-10 w-10 -translate-y-1/2 place-items-center rounded-full bg-white/10 text-white/85 backdrop-blur transition hover:bg-white/20 disabled:pointer-events-none disabled:opacity-25"
        :class="controlsVisible ? 'opacity-100' : 'pointer-events-none opacity-0'"
        title="Next photo (→)"
        :disabled="store.selectedIndex < 0 || store.selectedIndex >= store.assets.length - 1"
        @click="next"
      >
        <ChevronRight class="h-5 w-5" />
      </button>

      <div
        class="pointer-events-none absolute inset-x-0 bottom-0 flex items-center justify-center gap-4 bg-gradient-to-t from-black/70 to-transparent px-5 pb-4 pt-12 text-2xs text-white/60 transition-opacity duration-200"
        :class="controlsVisible ? 'opacity-100' : 'opacity-0'"
      >
        <span class="tabular-nums">
          {{ store.selectedIndex + 1 }} / {{ formatCount(store.assets.length) }}
        </span>
        <span class="hidden sm:inline">← → navigates · Esc exits</span>
      </div>
    </div>
  </div>
</template>
