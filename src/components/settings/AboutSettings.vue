<script setup lang="ts">
import { Keyboard } from 'lucide-vue-next';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

function openShortcuts() {
  store.closeSettings();
  store.openShortcuts();
}
</script>

<template>
  <SettingsSection title="About Lumio">
    <div class="px-3.5 py-3">
      <div class="flex items-center justify-between">
        <p class="text-xs font-medium text-ink">Lumio {{ store.appVersion }}</p>
        <span class="badge bg-panel-2 text-muted ring-1 ring-line">GPL-3.0-or-later</span>
      </div>
      <p class="mt-1.5 text-2xs leading-relaxed text-subtle">
        Local-first RAW photo browser. No cloud, no modifications to originals. Ingest copies RAW,
        JPEG and XMP files into a structured destination and can write IPTC metadata and keywords
        to the copies.
      </p>
      <p class="mt-2 text-2xs text-subtle">
        Supported: RAW (via rawler) · JPEG · PNG · TIFF · WebP. HEIC and AVIF are planned.
      </p>
      <button class="btn mt-2.5" @click="openShortcuts()">
        <Keyboard class="h-3.5 w-3.5" />
        Keyboard shortcuts
      </button>
    </div>
  </SettingsSection>
</template>
