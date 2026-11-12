<script setup lang="ts">
import { EyeOff, Loader2, RotateCcw } from 'lucide-vue-next';
import { ref } from 'vue';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { formatCount, pluralize } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const restoringId = ref<string | null>(null);

async function restoreHidden(id: string) {
  if (restoringId.value) return;
  restoringId.value = id;
  await store.restoreHiddenAssets([id]);
  restoringId.value = null;
}

async function restoreAllHidden() {
  if (restoringId.value || !store.hiddenAssets.length) return;
  restoringId.value = 'all';
  await store.restoreHiddenAssets(store.hiddenAssets.map((asset) => asset.id));
  restoringId.value = null;
}
</script>

<template>
  <SettingsSection
    title="Hidden photos"
    description="Photos removed from the library stay on disk and can be restored here."
  >
    <template #actions>
      <button
        v-if="store.hiddenAssets.length > 1"
        class="btn text-2xs"
        :disabled="restoringId !== null"
        @click="restoreAllHidden()"
      >
        <Loader2 v-if="restoringId === 'all'" class="h-3 w-3 animate-spin" />
        <RotateCcw v-else class="h-3 w-3" />
        Restore all
      </button>
    </template>

    <div v-if="!store.hiddenAssets.length" class="flex items-start gap-2.5 px-3.5 py-3.5">
      <EyeOff class="mt-0.5 h-4 w-4 shrink-0 text-subtle" />
      <div>
        <p class="text-xs font-medium text-ink">No hidden photos</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          Photos removed with “Delete from library only” appear here. Their files stay on disk and
          can be restored at any time.
        </p>
      </div>
    </div>
    <div v-else class="divide-y divide-line">
      <div
        v-for="hidden in store.hiddenAssets"
        :key="hidden.id"
        class="flex items-center gap-3 px-3.5 py-3"
      >
        <div class="min-w-0 flex-1">
          <p class="truncate text-xs font-medium text-ink">{{ hidden.name }}</p>
          <p class="mt-0.5 break-all font-mono text-2xs text-subtle">
            {{ hidden.primaryPath }}
          </p>
          <p class="mt-0.5 text-2xs text-subtle">
            {{ formatCount(hidden.paths.length) }}
            {{ pluralize(hidden.paths.length, 'file') }} hidden
          </p>
        </div>
        <button
          class="btn shrink-0 text-2xs"
          :disabled="restoringId !== null"
          @click="restoreHidden(hidden.id)"
        >
          <Loader2 v-if="restoringId === hidden.id" class="h-3 w-3 animate-spin" />
          <RotateCcw v-else class="h-3 w-3" />
          Restore
        </button>
      </div>
    </div>
  </SettingsSection>
</template>
