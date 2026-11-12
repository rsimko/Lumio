<script setup lang="ts">
import SettingsSection from '@/components/settings/SettingsSection.vue';
import SettingsToggle from '@/components/settings/SettingsToggle.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
</script>

<template>
  <SettingsSection
    title="Library browsing"
    description="Defaults for how folders and the surrounding panels behave."
  >
    <div class="divide-y divide-line">
      <SettingsToggle
        title="Include subfolders"
        description="Scan nested folders when a folder is opened."
        :model-value="store.settings.recursive"
        @update:model-value="store.setRecursive"
      />
      <SettingsToggle
        title="Show info panel"
        description="File details, camera data, metadata and associated files."
        :model-value="store.infoVisible"
        @update:model-value="store.toggleInfo()"
      />
      <SettingsToggle
        v-if="store.viewMode === 'filmstrip'"
        title="Show filmstrip"
        description="Keep the navigation strip visible below the photo."
        :model-value="store.settings.filmstripVisible"
        @update:model-value="store.setFilmstripVisible"
      />
    </div>
  </SettingsSection>
</template>
