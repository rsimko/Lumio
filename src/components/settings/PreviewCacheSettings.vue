<script setup lang="ts">
import { Loader2, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const confirmingClear = ref(false);

async function clearCache() {
  if (!confirmingClear.value) {
    confirmingClear.value = true;
    return;
  }
  await store.clearThumbnailCache();
  confirmingClear.value = false;
}
</script>

<template>
  <SettingsSection
    title="Preview cache"
    description="Thumbnails are generated locally and can be rebuilt at any time."
  >
    <div class="px-3.5 py-3">
      <p class="text-2xs uppercase tracking-wider text-subtle">Cache location</p>
      <p class="mt-1 break-all font-mono text-2xs text-muted selectable">{{ store.cacheDir }}</p>
      <p class="mt-2 text-2xs leading-relaxed text-subtle">
        Clearing the cache never touches your photo files.
      </p>
      <div class="mt-2.5 flex items-center gap-2">
        <button
          class="btn text-danger hover:border-danger/40 hover:bg-danger/10"
          :disabled="store.clearingCache"
          @click="clearCache()"
        >
          <Loader2 v-if="store.clearingCache" class="h-3.5 w-3.5 animate-spin" />
          <Trash2 v-else class="h-3.5 w-3.5" />
          {{ confirmingClear ? 'Confirm clear cache' : 'Clear thumbnail cache' }}
        </button>
        <button v-if="confirmingClear" class="btn" @click="confirmingClear = false">
          Cancel
        </button>
      </div>
    </div>
  </SettingsSection>
</template>
