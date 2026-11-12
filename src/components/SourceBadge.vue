<script setup lang="ts">
import { computed } from 'vue';

import { previewSourceBadge, sourceDescription } from '@/lib/format';
import type { AssetKind, PreviewSource } from '@/types';

const props = withDefaults(
  defineProps<{
    source: PreviewSource;
    kind?: AssetKind;
    /** Solid dark treatment for badges drawn over photo pixels. */
    overlay?: boolean;
  }>(),
  { overlay: false },
);

const label = computed(() => previewSourceBadge(props.source, props.kind));

const classes = computed(() => {
  if (props.overlay) {
    return 'bg-black/60 text-white/90 ring-white/20 backdrop-blur-sm';
  }
  switch (props.source) {
    case 'companionJpeg':
      return 'bg-success/15 text-success ring-success/20';
    case 'embeddedRaw':
      return 'bg-warning/15 text-warning ring-warning/25';
    case 'original':
      return 'bg-panel-2 text-muted ring-line';
    case 'placeholder':
    default:
      return 'bg-danger/15 text-danger ring-danger/20';
  }
});
</script>

<template>
  <span class="badge ring-1" :class="classes" :title="sourceDescription(source)">
    {{ label }}
  </span>
</template>
