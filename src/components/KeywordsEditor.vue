<script setup lang="ts">
import { computed } from 'vue';

import InfoSection from '@/components/InfoSection.vue';
import IngestKeywordEditor from '@/components/IngestKeywordEditor.vue';
import MetadataSavedBadge from '@/components/MetadataSavedBadge.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const selection = computed(() => store.selectedAssets);
const single = computed(() => (selection.value.length === 1 ? selection.value[0] : null));
const keywords = computed(() => single.value?.metadata.keywords ?? []);
const hierarchical = computed(() => single.value?.metadata.hierarchicalKeywords ?? []);

function updateKeywords(value: string[]): void {
  if (!single.value) return;
  void store.applyMetadataPatch({ keywords: value }, [single.value], { silent: true });
}
</script>

<template>
  <InfoSection title="Keywords">
    <template #actions>
      <MetadataSavedBadge />
    </template>

    <div v-if="single" class="space-y-2">
      <IngestKeywordEditor :model-value="keywords" @update:model-value="updateKeywords" />

      <div v-if="hierarchical.length" class="border-t border-line/60 pt-2">
        <div class="mb-1 flex items-center justify-between">
          <span class="text-2xs text-subtle">Hierarchical</span>
          <span class="text-3xs uppercase tracking-wide text-subtle/70">Read-only</span>
        </div>
        <div class="flex flex-wrap gap-1">
          <span
            v-for="keyword in hierarchical"
            :key="`hier-${keyword}`"
            class="rounded bg-accent-soft px-1.5 py-0.5 text-2xs text-accent"
            :title="`Hierarchical keyword: ${keyword}`"
          >
            {{ keyword.replaceAll('|', ' › ') }}
          </span>
        </div>
      </div>
    </div>

    <p v-else class="text-2xs leading-relaxed text-subtle">
      Keywords are edited one photo at a time. Select a single photo to change its keywords.
    </p>
  </InfoSection>
</template>
