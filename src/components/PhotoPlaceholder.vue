<script setup lang="ts">
import { AlertTriangle, Loader2, Mountain } from 'lucide-vue-next';

import type { PhotoAsset } from '@/types';

withDefaults(
  defineProps<{
    asset?: PhotoAsset | null;
    loading?: boolean;
    error?: string | null;
    compact?: boolean;
  }>(),
  {
    asset: null,
    loading: false,
    error: null,
    compact: false,
  },
);
</script>

<template>
  <div
    class="flex h-full w-full flex-col items-center justify-center gap-1 overflow-hidden bg-canvas text-subtle"
    :class="loading ? 'animate-pulse' : ''"
  >
    <Loader2 v-if="loading" class="h-4 w-4 animate-spin opacity-70" />
    <AlertTriangle v-else-if="error" class="h-4 w-4 text-danger/80" />
    <Mountain v-else class="h-4 w-4 opacity-50" />
    <span
      v-if="asset && !compact"
      class="text-2xs font-semibold uppercase tracking-wider opacity-60"
    >
      {{ asset.extension }}
    </span>
  </div>
</template>
