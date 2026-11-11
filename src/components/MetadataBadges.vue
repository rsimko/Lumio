<script setup lang="ts">
import { Flag, Heart, Star, X } from 'lucide-vue-next';
import { computed } from 'vue';

import {
  COLOR_LABEL_NAMES,
  COLOR_LABEL_SWATCHES,
  hasVisibleMetadata,
} from '@/lib/metadata';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    /** Overlay chips are drawn over photo pixels; inline badges sit in a row. */
    variant?: 'overlay' | 'inline';
  }>(),
  { variant: 'overlay' },
);

const metadata = computed(() => props.asset.metadata);
const visible = computed(() => hasVisibleMetadata(metadata.value));
const overlay = computed(() => props.variant === 'overlay');

const chipClass = computed(() =>
  overlay.value
    ? 'rounded bg-black/55 px-1 py-0.5 text-white/90 backdrop-blur-[2px]'
    : 'rounded px-1 py-0.5',
);
</script>

<template>
  <span
    v-if="visible"
    class="flex items-center gap-1 text-3xs leading-none"
    :class="overlay ? 'drop-shadow-sm' : 'text-muted'"
    aria-label="Photo metadata"
  >
    <span
      v-if="metadata.rating > 0"
      class="flex items-center gap-0.5 font-semibold tabular-nums"
      :class="[chipClass, overlay ? 'text-amber-300' : 'text-amber-500']"
      :title="`${metadata.rating} ${metadata.rating === 1 ? 'star' : 'stars'}`"
    >
      <Star class="h-2.5 w-2.5" fill="currentColor" />
      {{ metadata.rating }}
    </span>
    <span
      v-if="metadata.label"
      class="h-2 w-2 shrink-0 rounded-full ring-1 ring-black/25"
      :class="COLOR_LABEL_SWATCHES[metadata.label]"
      :title="`${COLOR_LABEL_NAMES[metadata.label]} label`"
    />
    <span
      v-if="metadata.favorite"
      :class="[chipClass, overlay ? 'text-rose-300' : 'text-rose-500']"
      title="Favorite"
    >
      <Heart class="h-2.5 w-2.5" fill="currentColor" />
    </span>
    <span
      v-if="metadata.cull === 'pick'"
      :class="[chipClass, overlay ? 'text-emerald-300' : 'text-success']"
      title="Picked"
    >
      <Flag class="h-2.5 w-2.5" fill="currentColor" />
    </span>
    <span
      v-else-if="metadata.cull === 'reject'"
      :class="[chipClass, overlay ? 'text-red-300' : 'text-danger']"
      title="Rejected"
    >
      <X class="h-2.5 w-2.5" />
    </span>
  </span>
</template>
