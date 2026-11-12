<script setup lang="ts">
import { AppWindow, Loader2, Plus, Star, Trash2 } from 'lucide-vue-next';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

function addEditor() {
  void store.addExternalEditor();
}
</script>

<template>
  <SettingsSection
    title="External editors"
    description="Applications offered by “Edit with” in the photo context menu."
  >
    <template #actions>
      <button class="btn text-2xs" :disabled="store.addingEditor" @click="addEditor()">
        <Loader2 v-if="store.addingEditor" class="h-3 w-3 animate-spin" />
        <Plus v-else class="h-3 w-3" />
        Add application
      </button>
    </template>

    <div v-if="!store.externalEditors.length" class="flex items-start gap-2.5 px-3.5 py-3.5">
      <AppWindow class="mt-0.5 h-4 w-4 shrink-0 text-subtle" />
      <div>
        <p class="text-xs font-medium text-ink">No external editors</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          Add applications such as Photoshop or Lightroom to open photos from Lumio. The default
          editor is used by the
          <span class="font-medium text-muted">
            {{ store.shortcutLabel('browse.editDefault') }}
          </span>
          shortcut.
        </p>
      </div>
    </div>
    <div v-else class="divide-y divide-line">
      <div
        v-for="editor in store.externalEditors"
        :key="editor.path"
        class="flex items-center gap-3 px-3.5 py-3"
      >
        <AppWindow class="h-4 w-4 shrink-0 text-subtle" />
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-1.5">
            <p class="truncate text-xs font-medium text-ink">{{ editor.name }}</p>
            <span
              v-if="editor.path === store.defaultEditor?.path"
              class="shrink-0 rounded bg-accent-soft px-1 py-0.5 text-3xs font-semibold uppercase tracking-wide text-accent"
            >
              Default
            </span>
          </div>
          <p class="mt-0.5 break-all font-mono text-2xs text-subtle">{{ editor.path }}</p>
        </div>
        <button
          v-if="editor.path !== store.defaultEditor?.path"
          class="btn shrink-0 text-2xs"
          :title="`Use ${editor.name} for the Edit shortcut`"
          @click="store.setDefaultEditor(editor.path)"
        >
          <Star class="h-3 w-3" />
          Make default
        </button>
        <button
          class="btn-icon-sm shrink-0"
          :title="`Remove ${editor.name}`"
          @click="store.removeExternalEditor(editor.path)"
        >
          <Trash2 class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <template v-if="store.externalEditors.length" #footer>
      <p class="mt-1.5 text-2xs leading-relaxed text-subtle">
        The default editor opens with the
        <span class="font-medium text-muted">
          {{ store.shortcutLabel('browse.editDefault') }}
        </span>
        shortcut.
      </p>
    </template>
  </SettingsSection>
</template>
