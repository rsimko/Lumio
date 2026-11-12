<script setup lang="ts">
import { reactive, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { SETTINGS_TABS } from '@/components/settings/registry';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const firstTab = SETTINGS_TABS[0]!;
const activeTabId = ref(firstTab.id);
const visitedTabs = reactive(new Set<string>([firstTab.id]));

function selectTab(id: string) {
  visitedTabs.add(id);
  activeTabId.value = id;
}
</script>

<template>
  <AppDialog
    title="Settings"
    subtitle="Preferences are stored locally on this Mac."
    xl
    flush
    @close="store.closeSettings()"
  >
    <div class="flex min-h-0 flex-1">
      <nav
        class="flex w-52 shrink-0 flex-col border-r border-line bg-panel/55 px-2.5 py-3"
        aria-label="Settings sections"
      >
        <div class="space-y-0.5">
          <button
            v-for="tab in SETTINGS_TABS"
            :key="tab.id"
            class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs transition"
            :class="
              activeTabId === tab.id
                ? 'bg-elevated font-medium text-ink shadow-sm ring-1 ring-line'
                : 'text-muted hover:bg-panel-2 hover:text-ink'
            "
            :aria-current="activeTabId === tab.id ? 'page' : undefined"
            @click="selectTab(tab.id)"
          >
            <component
              :is="tab.icon"
              class="h-4 w-4 shrink-0"
              :class="activeTabId === tab.id ? 'text-accent' : 'text-subtle'"
            />
            <span class="truncate">{{ tab.label }}</span>
          </button>
        </div>
        <p class="mt-auto px-2.5 pt-4 text-3xs text-subtle">Lumio {{ store.appVersion }}</p>
      </nav>

      <div class="min-h-0 min-w-0 flex-1">
        <div
          v-for="tab in SETTINGS_TABS"
          v-show="tab.id === activeTabId"
          :key="tab.id"
          class="h-full overflow-y-auto"
        >
          <div class="mx-auto max-w-[660px] px-6 py-5">
            <header class="mb-5 border-b border-line/70 pb-4">
              <h2 class="text-sm font-semibold text-ink">{{ tab.title }}</h2>
              <p class="mt-1 text-xs leading-relaxed text-muted">{{ tab.description }}</p>
            </header>
            <div v-if="visitedTabs.has(tab.id)" class="space-y-7">
              <component
                v-for="(section, index) in tab.sections"
                :is="section"
                :key="`${tab.id}-${index}`"
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <button class="btn" @click="store.closeSettings()">Close</button>
    </template>
  </AppDialog>
</template>
