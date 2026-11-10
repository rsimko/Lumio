<script setup lang="ts">
import { Images, Keyboard, Layers, Map, Settings } from 'lucide-vue-next';

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
      class="flex h-8 w-8 items-center justify-center rounded-md bg-accent-soft text-accent"
      title="Library"
      aria-current="page"
    >
      <Images class="h-4 w-4" />
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
        title="Keyboard shortcuts (?)"
        @click="store.openShortcuts()"
      >
        <Keyboard class="h-4 w-4" />
      </button>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-md text-subtle transition hover:bg-panel-2 hover:text-muted"
        title="Settings (⌘,)"
        @click="store.openSettings()"
      >
        <Settings class="h-4 w-4" />
      </button>
    </div>
  </nav>
</template>
