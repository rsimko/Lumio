<script setup lang="ts">
import { ExternalLink, MapPin } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import { api, errorMessage } from '@/api';
import { formatCoordinates, hasCoordinates, mapPosition } from '@/lib/geo';
import { useLibraryStore } from '@/stores/library';

const props = defineProps<{
  latitude: number | null;
  longitude: number | null;
}>();

const store = useLibraryStore();
const opening = ref(false);

const available = computed(() => hasCoordinates(props.latitude, props.longitude));
const position = computed(() => mapPosition(props.latitude, props.longitude));
const caption = computed(() => formatCoordinates(props.latitude, props.longitude));

async function openInMaps(): Promise<void> {
  if (!available.value || opening.value) return;
  opening.value = true;
  try {
    await api.openMaps(props.latitude as number, props.longitude as number);
  } catch (error) {
    store.addToast(errorMessage(error), 'error');
  } finally {
    opening.value = false;
  }
}
</script>

<template>
  <div
    class="relative h-28 select-none overflow-hidden rounded-md border border-line bg-panel-2/70 ring-1 ring-line/40"
    role="img"
    :aria-label="available ? `Map preview for ${caption}` : 'Location map placeholder'"
  >
    <svg class="absolute inset-0 h-full w-full text-line/70" aria-hidden="true">
      <defs>
        <pattern id="lumio-geo-grid" width="18" height="18" patternUnits="userSpaceOnUse">
          <path d="M18 0H0V18" fill="none" stroke="currentColor" stroke-width="0.5" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#lumio-geo-grid)" />
      <circle
        cx="28%"
        cy="34%"
        r="24%"
        fill="none"
        stroke="currentColor"
        stroke-width="0.75"
        stroke-dasharray="2 4"
        opacity="0.7"
      />
      <circle
        cx="72%"
        cy="68%"
        r="17%"
        fill="none"
        stroke="currentColor"
        stroke-width="0.75"
        stroke-dasharray="2 4"
        opacity="0.55"
      />
      <path
        d="M-8 78 C 22 58, 42 92, 68 70 S 108 42, 128 58"
        fill="none"
        stroke="currentColor"
        stroke-width="0.6"
        opacity="0.5"
      />
    </svg>

    <div
      class="absolute transition-all duration-300"
      :class="available ? '' : 'opacity-40'"
      :style="{ left: `${position.left}%`, top: `${position.top}%` }"
    >
      <span
        v-if="available"
        class="absolute left-1/2 top-0 h-3 w-3 -translate-x-1/2 -translate-y-1/2 animate-ping rounded-full bg-accent/30"
      />
      <MapPin
        class="relative h-5 w-5 -translate-x-1/2 -translate-y-full text-accent drop-shadow-sm"
        fill="currentColor"
        :stroke-width="1.5"
      />
    </div>

    <div class="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-between gap-2 bg-gradient-to-t from-panel/90 to-transparent px-2 pb-1.5 pt-5">
      <span class="min-w-0 truncate font-mono text-[10px] text-muted">
        {{ available ? caption : 'No coordinates set' }}
      </span>
      <span class="text-3xs uppercase tracking-wider text-subtle/70">
        {{ available ? 'Offline preview' : 'Map placeholder' }}
      </span>
    </div>

    <button
      v-if="available"
      class="absolute right-1.5 top-1.5 inline-flex h-6 items-center gap-1 rounded border border-line bg-panel/90 px-1.5 text-2xs font-medium text-ink shadow-sm transition hover:border-line-strong hover:bg-elevated disabled:pointer-events-none disabled:opacity-50"
      :disabled="opening"
      title="Open these coordinates in Maps"
      @click="openInMaps"
    >
      <ExternalLink class="h-3 w-3" />
      Open in Maps
    </button>
  </div>
</template>
