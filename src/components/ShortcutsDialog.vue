<script setup lang="ts">
import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

interface ShortcutRow {
  keys: string[];
  action: string;
}

interface ShortcutGroup {
  title: string;
  rows: ShortcutRow[];
}

const groups: ShortcutGroup[] = [
  {
    title: 'Browsing',
    rows: [
      { keys: ['←', '→'], action: 'Move selection' },
      { keys: ['↑', '↓'], action: 'Move by row / item' },
      { keys: ['⇧', '← → ↑ ↓'], action: 'Extend selection' },
      { keys: ['⌘', 'Click'], action: 'Toggle a photo in the selection' },
      { keys: ['⇧', 'Click'], action: 'Select a range of photos' },
      { keys: ['⌘', 'A'], action: 'Select all photos' },
      { keys: ['Delete'], action: 'Delete selected photos' },
      { keys: ['Right-click'], action: 'Delete a photo or the selected photos' },
      { keys: ['Drag'], action: 'Move photos onto a folder row' },
      { keys: ['Space', 'Enter'], action: 'Open fullscreen viewer' },
      { keys: ['Double-click'], action: 'Open a thumbnail in filmstrip view' },
      { keys: ['Esc'], action: 'Close viewer, dialog or clear selection' },
      { keys: ['G'], action: 'Grid view' },
      { keys: ['M'], action: 'Masonry view' },
      { keys: ['J'], action: 'Justified view' },
      { keys: ['L'], action: 'List view' },
      { keys: ['F'], action: 'Filmstrip view' },
      { keys: ['+', '−'], action: 'Change thumbnail size' },
      { keys: ['0'], action: 'Reset thumbnail size' },
    ],
  },
  {
    title: 'Library',
    rows: [
      { keys: ['⌘', 'O'], action: 'Add a folder' },
      { keys: ['⌘', 'R'], action: 'Refresh the selected folder' },
      { keys: ['R'], action: 'Toggle include subfolders' },
      { keys: ['B'], action: 'Toggle filmstrip (filmstrip view only)' },
      { keys: ['I'], action: 'Toggle info panel' },
      { keys: ['T'], action: 'Toggle light / dark appearance' },
      { keys: ['⌘', ','], action: 'Open settings' },
      { keys: ['?'], action: 'Show this shortcut reference' },
    ],
  },
  {
    title: 'Fullscreen',
    rows: [
      { keys: ['←', '→'], action: 'Previous / next photo' },
      { keys: ['Space'], action: 'Exit fullscreen' },
      { keys: ['Esc'], action: 'Exit fullscreen' },
    ],
  },
];
</script>

<template>
  <AppDialog
    title="Keyboard shortcuts"
    subtitle="Lumio stays fast without leaving the keyboard."
    wide
    @close="store.closeShortcuts()"
  >
    <div class="grid gap-6 sm:grid-cols-2">
      <section v-for="group in groups" :key="group.title">
        <h3 class="panel-heading mb-2">{{ group.title }}</h3>
        <dl class="space-y-1.5">
          <div
            v-for="row in group.rows"
            :key="row.action"
            class="flex items-center justify-between gap-3"
          >
            <dt class="text-xs text-muted">{{ row.action }}</dt>
            <dd class="flex shrink-0 items-center gap-1">
              <kbd
                v-for="key in row.keys"
                :key="key"
                class="min-w-[20px] rounded border border-line bg-panel px-1.5 py-0.5 text-center font-mono text-2xs text-muted shadow-sm"
              >
                {{ key }}
              </kbd>
            </dd>
          </div>
        </dl>
      </section>
    </div>

    <template #footer>
      <button class="btn" @click="store.closeShortcuts()">Close</button>
    </template>
  </AppDialog>
</template>
