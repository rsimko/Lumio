<script setup lang="ts">
import { AlertTriangle, Loader2 } from 'lucide-vue-next';
import { computed } from 'vue';

import { formatCount, formatDuration, pluralize } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const leftText = computed(() => {
  if (store.selectedCount > 1) return `${store.selectedCount} photos selected`;
  if (store.selectedAsset) return store.selectedAsset.fileName;
  if (store.selectedFolder) return store.relativeFolderLabel;
  if (store.initializing) return 'Opening library…';
  return 'Ready';
});

const statusDot = computed(() => {
  if (store.initializationError || store.scanState === 'error') return 'bg-danger';
  if (store.scanState === 'loading') return 'bg-accent animate-pulse';
  if (store.scanState === 'ready') return 'bg-success';
  return 'bg-subtle';
});
</script>

<template>
  <footer
    class="flex h-7 shrink-0 items-center gap-3 border-t border-line bg-panel px-3 text-2xs text-subtle"
  >
    <div class="flex min-w-0 flex-1 items-center gap-1.5">
      <span class="h-1.5 w-1.5 shrink-0 rounded-full" :class="statusDot" />
      <Loader2
        v-if="store.scanState === 'loading'"
        class="h-3 w-3 shrink-0 animate-spin"
      />
      <span class="truncate">{{ leftText }}</span>
    </div>

    <div class="flex shrink-0 items-center gap-3">
      <span v-if="store.selectedCount > 1" class="font-medium text-accent">
        {{ formatCount(store.selectedCount) }} selected
      </span>
      <span v-if="store.selectedIndex >= 0" class="tabular-nums">
        {{ store.selectedIndex + 1 }} / {{ formatCount(store.assets.length) }}
      </span>
      <span v-else-if="store.scanState === 'ready'" class="tabular-nums">
        {{ formatCount(store.assets.length) }}
        {{ pluralize(store.assets.length, 'photo') }}<span v-if="store.hasActiveFilters">
          of {{ formatCount(store.totalCount) }}</span
        >
      </span>
      <span v-if="store.hasActiveFilters" class="font-medium text-accent">Filtered</span>
      <span v-if="store.settings.recursive" class="text-accent">Recursive</span>
      <span v-if="store.scanState === 'ready' && store.scanElapsedMs" class="hidden tabular-nums md:inline">
        Scanned in {{ formatDuration(store.scanElapsedMs) }}
      </span>
      <button
        v-if="store.warningsCount"
        class="flex items-center gap-1 rounded px-1 py-0.5 text-warning transition hover:bg-warning/10"
        title="Show scan warnings"
        @click="store.openWarnings()"
      >
        <AlertTriangle class="h-3 w-3" />
        {{ store.warningsCount }}
        {{ store.warningsCount === 1 ? 'warning' : 'warnings' }}
      </button>
    </div>
  </footer>
</template>
