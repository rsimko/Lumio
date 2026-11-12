<script setup lang="ts">
import { Check, Monitor, Moon, Sun } from 'lucide-vue-next';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { useLibraryStore } from '@/stores/library';
import type { ThemeSetting } from '@/types';

const store = useLibraryStore();

const themes: { value: ThemeSetting; label: string; icon: typeof Monitor }[] = [
  { value: 'system', label: 'System', icon: Monitor },
  { value: 'light', label: 'Light', icon: Sun },
  { value: 'dark', label: 'Dark', icon: Moon },
];
</script>

<template>
  <SettingsSection
    title="Theme"
    description="Choose how Lumio looks. System follows the macOS appearance automatically."
  >
    <div class="grid grid-cols-3 gap-1.5 p-3">
      <button
        v-for="theme in themes"
        :key="theme.value"
        class="flex items-center justify-center gap-1.5 rounded-md border px-2 py-2 text-xs transition"
        :class="
          store.settings.theme === theme.value
            ? 'border-accent bg-accent-soft font-medium text-ink'
            : 'border-line bg-panel text-muted hover:bg-panel-2'
        "
        :aria-pressed="store.settings.theme === theme.value"
        @click="store.setTheme(theme.value)"
      >
        <component :is="theme.icon" class="h-3.5 w-3.5" />
        {{ theme.label }}
        <Check v-if="store.settings.theme === theme.value" class="h-3 w-3 text-accent" />
      </button>
    </div>
  </SettingsSection>
</template>
