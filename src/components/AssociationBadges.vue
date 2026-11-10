<script setup lang="ts">
import { computed } from 'vue';

import { basename } from '@/lib/format';
import type { PhotoAsset } from '@/types';

const props = defineProps<{
  asset: PhotoAsset;
}>();

const jpegTitle = computed(() => {
  const path = props.asset.companionJpeg;
  if (!path) return '';
  const extra = props.asset.extraCompanions.length;
  return `Companion JPEG: ${basename(path)}${extra ? ` (+${extra} more)` : ''}`;
});

const xmpTitle = computed(() => {
  const path = props.asset.xmpSidecar;
  if (!path) return '';
  const extra = props.asset.extraSidecars.length;
  return `XMP sidecar: ${basename(path)}${extra ? ` (+${extra} more)` : ''}`;
});
</script>

<template>
  <span
    v-if="asset.companionJpeg"
    class="badge bg-success/15 text-success ring-1 ring-success/20"
    :title="jpegTitle"
  >
    JPG
  </span>
  <span
    v-if="asset.xmpSidecar"
    class="badge bg-accent-soft text-accent ring-1 ring-accent/20"
    :title="xmpTitle"
  >
    XMP
  </span>
</template>
