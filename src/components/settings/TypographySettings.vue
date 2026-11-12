<script setup lang="ts">
import FontField from '@/components/settings/FontField.vue';
import SettingsSection from '@/components/settings/SettingsSection.vue';
import SettingsSlider from '@/components/settings/SettingsSlider.vue';
import { useSystemFonts } from '@/composables/useSystemFonts';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const { fonts, loading, error } = useSystemFonts();
</script>

<template>
  <SettingsSection
    title="Typography"
    description="Pick any installed family or type one manually. Avenir and bundled JetBrains Mono remain the defaults."
  >
    <div class="divide-y divide-line">
      <FontField
        label="Interface font"
        description="Used for labels, menus and panels across the whole app."
        kind="ui"
        :model-value="store.settings.uiFont"
        :fonts="fonts"
        :loading="loading"
        :error="error"
        @update:model-value="store.setUiFont"
      />
      <FontField
        label="Monospace font"
        description="Used for file paths, cache locations and keyboard shortcuts."
        kind="mono"
        :model-value="store.settings.monoFont"
        :fonts="fonts"
        :loading="loading"
        :error="error"
        @update:model-value="store.setMonoFont"
      />
      <SettingsSlider
        title="Text size"
        description="Scales labels and text without resizing panels."
        :model-value="store.settings.uiFontSize"
        :min="11"
        :max="17"
        :step="1"
        :value-label="`${store.settings.uiFontSize} px`"
        @update:model-value="store.setUiFontSize"
      />
      <div class="px-3.5 py-3">
        <p class="text-2xs uppercase tracking-wider text-subtle">Preview</p>
        <p class="mt-1.5 rounded-md border border-line/70 bg-elevated px-2.5 py-2 text-xs text-muted">
          <span class="text-ink">The quick brown fox</span>
          <span class="text-subtle"> · </span>
          <span class="font-mono text-2xs">ISO 100 · 35 mm</span>
        </p>
      </div>
    </div>
  </SettingsSection>
</template>
