<script setup lang="ts">
import { Funnel, Images, Keyboard, Layers, Map, Settings } from 'lucide-vue-next';

import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

function planned(feature: string) {
  store.addToast(`${feature} is planned for a later milestone.`, 'info');
}
</script>

<template>
  <nav
    class="flex w-[46px] shrink-0 flex-col items-center gap-1 border-r border-line bg-panel py-2"
    aria-label="Workspace tabs"
  >
    <button
      class="flex h-8 w-8 items-center justify-center rounded-md transition"
      :class="
        store.leftPanelTab === 'library'
          ? 'bg-accent-soft text-accent'
          : 'text-subtle hover:bg-panel-2 hover:text-muted'
      "
      title="Library"
      :aria-current="store.leftPanelTab === 'library' ? 'page' : undefined"
      @click="store.setLeftPanelTab('library')"
    >
      <Images class="h-4 w-4" />
    </button>
    <button
      class="relative flex h-8 w-8 items-center justify-center rounded-md transition"
      :class="
        store.leftPanelTab === 'filters'
          ? 'bg-accent-soft text-accent'
          : 'text-subtle hover:bg-panel-2 hover:text-muted'
      "
      title="Filters"
      :aria-current="store.leftPanelTab === 'filters' ? 'page' : undefined"
      @click="store.setLeftPanelTab('filters')"
    >
      <Funnel class="h-4 w-4" />
      <span
        v-if="store.hasActiveFilters"
        class="absolute -right-0.5 -top-0.5 min-w-[14px] rounded-full bg-accent px-1 text-center text-[9px] font-semibold leading-[14px] text-accent-ink"
      >
        {{ store.activeFilterCount }}
      </span>
    </button>
    <button
      class="flex h-8 w-8 items-center justify-center rounded-md text-subtle transition hover:bg-panel-2 hover:text-muted"
      title="Albums — planned"
      @click="planned('Albums')"
    >
      <Layers class="h-4 w-4" />
    </button>
    <button
      class="flex h-8 w-8 items-center justify-center rounded-md text-subtle transition hover:bg-panel-2 hover:text-muted"
      title="Map — planned"
      @click="planned('Map view')"
    >
      <Map class="h-4 w-4" />
    </button>

    <div class="my-1 h-px w-5 bg-line" />

    <div class="mt-auto flex flex-col items-center gap-1">
      <button
        class="flex h-8 w-8 items-center justify-center rounded-md text-subtle transition hover:bg-panel-2 hover:text-muted"
        :title="`Keyboard shortcuts (${store.shortcutLabel('help.shortcuts')})`"
        @click="store.openShortcuts()"
      >
        <Keyboard class="h-4 w-4" />
      </button>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-md text-subtle transition hover:bg-panel-2 hover:text-muted"
        :title="`Settings (${store.shortcutLabel('library.settings')})`"
        @click="store.openSettings()"
      >
        <Settings class="h-4 w-4" />
      </button>
    </div>
  </nav>
</template>
