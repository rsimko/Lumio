<script setup lang="ts">
import { FileImage, FileText } from 'lucide-vue-next';
import { computed } from 'vue';

import type { AssetAssociation } from '@/lib/assets';
import { basename, parentPath } from '@/lib/format';

const props = withDefaults(
  defineProps<{
    association: AssetAssociation;
    assetName: string;
    compact?: boolean;
  }>(),
  { compact: false },
);

const jpeg = computed(() => props.association.kind === 'jpeg');
const label = computed(() => {
  if (jpeg.value) {
    return props.association.extra ? 'Extra companion JPEG' : 'Companion JPEG';
  }
  return props.association.extra ? 'Extra XMP sidecar' : 'XMP sidecar';
});
const folder = computed(() => parentPath(props.association.path));
</script>

<template>
  <div
    class="relative flex h-full items-center gap-2 border-b border-line/40 bg-panel-2/40 pl-[66px] pr-4 text-xs"
    :aria-label="`${label}: ${association.path} (associated with ${assetName})`"
  >
    <span class="absolute left-[35px] top-0 h-full w-px bg-line-strong/45" aria-hidden="true" />
    <span class="absolute left-[35px] top-1/2 h-px w-[18px] bg-line-strong/45" aria-hidden="true" />
    <component
      :is="jpeg ? FileImage : FileText"
      class="h-3.5 w-3.5 shrink-0"
      :class="jpeg ? 'text-success' : 'text-accent'"
    />
    <span
      class="badge w-fit ring-1"
      :class="
        jpeg
          ? 'bg-success/15 text-success ring-success/20'
          : 'bg-accent-soft text-accent ring-accent/20'
      "
    >
      {{ jpeg ? 'JPG' : 'XMP' }}
    </span>
    <span class="min-w-0 truncate font-medium text-muted" :title="association.path">
      {{ basename(association.path) }}
    </span>
    <span v-if="!compact" class="shrink-0 text-2xs text-subtle">{{ label }}</span>
    <span
      v-if="!compact"
      class="ml-auto min-w-0 max-w-[40%] truncate font-mono text-2xs text-subtle/80"
      :title="folder"
    >
      {{ folder }}
    </span>
  </div>
</template>
