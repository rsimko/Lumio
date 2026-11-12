<script setup lang="ts">
import { computed } from 'vue';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import SettingsSlider from '@/components/settings/SettingsSlider.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const radiusLabel = computed(() =>
  store.settings.thumbnailRadius === 0 ? 'Sharp' : `${store.settings.thumbnailRadius} px`,
);
</script>

<template>
  <SettingsSection
    title="Thumbnails"
    description="Applies to grid, masonry, justified, list and filmstrip thumbnails."
  >
    <div class="divide-y divide-line">
      <SettingsSlider
        title="Thumbnail size"
        description="Maximum edge in grid, masonry and justified layouts."
        :model-value="store.settings.thumbnailSize"
        :min="120"
        :max="480"
        :step="4"
        :value-label="`${store.settings.thumbnailSize} px`"
        @update:model-value="store.setThumbnailSize"
      />
      <SettingsSlider
        title="Corner roundness"
        description="Turn the corners down to zero for a sharp, squared-off look."
        :model-value="store.settings.thumbnailRadius"
        :min="0"
        :max="16"
        :step="1"
        :value-label="radiusLabel"
        @update:model-value="store.setThumbnailRadius"
      />
      <div class="flex items-center gap-3 px-3.5 py-3">
        <div class="flex items-end gap-1.5">
          <div
            class="thumb-frame h-14 w-20 bg-gradient-to-br from-accent/45 via-accent/20 to-panel-2 ring-1 ring-black/[0.08] dark:ring-white/[0.1]"
          />
          <div
            class="thumb-frame h-10 w-14 bg-gradient-to-br from-panel-2 to-canvas ring-1 ring-black/[0.08] dark:ring-white/[0.1]"
          />
        </div>
        <div>
          <p class="text-xs font-medium text-ink">Live preview</p>
          <p class="mt-0.5 text-2xs text-subtle">Corners update as you drag.</p>
        </div>
      </div>
    </div>
  </SettingsSection>
</template>
