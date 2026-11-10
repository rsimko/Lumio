<script setup lang="ts">
import { AlertTriangle, RotateCcw } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import {
  SHORTCUT_DEFINITIONS,
  SHORTCUT_GROUPS,
  bindingFromEvent,
  formatBindingParts,
  validateShortcutBinding,
  type ShortcutDefinition,
  type ShortcutId,
} from '@/lib/shortcuts';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const editingId = ref<ShortcutId | null>(null);
const editError = ref<string | null>(null);

const groups = computed(() =>
  SHORTCUT_GROUPS.map((title) => ({
    title,
    rows: SHORTCUT_DEFINITIONS.filter((definition) => definition.group === title),
  })).filter((group) => group.rows.length > 0),
);

const hasCustomShortcuts = computed(() => Object.keys(store.settings.shortcuts).length > 0);

function partsFor(definition: ShortcutDefinition): string[] {
  return formatBindingParts(store.shortcutBinding(definition.id));
}

function aliasesFor(definition: ShortcutDefinition): string[] {
  const primary = store.shortcutBinding(definition.id);
  return (definition.aliases ?? []).filter((alias) => alias !== primary);
}

function aliasParts(alias: string): string[] {
  return formatBindingParts(alias);
}

function startEditing(definition: ShortcutDefinition): void {
  if (editingId.value === definition.id) {
    cancelEditing();
    return;
  }
  editingId.value = definition.id;
  editError.value = null;
}

function cancelEditing(): void {
  editingId.value = null;
  editError.value = null;
}

function resetAll(): void {
  cancelEditing();
  store.resetShortcuts();
}

function onCaptureKeydown(event: KeyboardEvent): void {
  const id = editingId.value;
  if (!id) return;
  // The shortcut dialog owns the keyboard while a binding is being captured.
  event.preventDefault();
  event.stopImmediatePropagation();

  if (event.key === 'Escape') {
    cancelEditing();
    return;
  }

  const binding = bindingFromEvent(event);
  if (!binding) return;

  const conflict = validateShortcutBinding(id, binding, store.settings.shortcuts);
  if (conflict) {
    editError.value = conflict;
    return;
  }

  store.setShortcut(id, binding);
  cancelEditing();
}

onMounted(() => {
  window.addEventListener('keydown', onCaptureKeydown, true);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onCaptureKeydown, true);
});
</script>

<template>
  <AppDialog
    title="Keyboard shortcuts"
    subtitle="Click a keycap, then press the new combination."
    wide
    @close="store.closeShortcuts()"
  >
    <p class="mb-3 text-2xs leading-relaxed text-subtle">
      Escape cancels while recording. Arrow keys, Escape and the fullscreen Delete action stay
      fixed so navigation always works.
    </p>

    <p
      v-if="editError"
      class="mb-3 flex items-start gap-1.5 rounded-md border border-danger/30 bg-danger/10 px-2.5 py-2 text-2xs leading-relaxed text-danger"
    >
      <AlertTriangle class="mt-px h-3 w-3 shrink-0" />
      {{ editError }}
    </p>

    <div class="grid gap-x-8 gap-y-5 sm:grid-cols-2">
      <section v-for="group in groups" :key="group.title">
        <h3 class="panel-heading mb-2">{{ group.title }}</h3>
        <dl class="space-y-1.5">
          <div
            v-for="row in group.rows"
            :key="row.id"
            class="flex items-center justify-between gap-3"
          >
            <dt class="text-xs leading-snug text-muted">{{ row.label }}</dt>
            <dd class="flex shrink-0 items-center gap-1">
              <button
                v-if="editingId === row.id"
                class="rounded border border-accent bg-accent-soft px-2 py-0.5 font-mono text-2xs text-accent shadow-sm"
                title="Press the new shortcut, or Escape to cancel"
                @click="cancelEditing"
              >
                Press keys…
              </button>
              <template v-else>
                <button
                  v-for="(part, index) in partsFor(row)"
                  :key="`${row.id}-${index}`"
                  class="min-w-[22px] rounded border border-line bg-panel px-1.5 py-0.5 text-center font-mono text-2xs shadow-sm transition hover:border-line-strong hover:text-ink"
                  :class="store.shortcutIsCustom(row.id) ? 'text-accent' : 'text-muted'"
                  :title="`Change “${row.label}”`"
                  :aria-label="`Change ${row.label}. Current shortcut: ${partsFor(row).join(' ')}`"
                  @click="startEditing(row)"
                >
                  {{ part }}
                </button>
                <kbd
                  v-for="alias in aliasesFor(row)"
                  :key="`${row.id}-alias-${alias}`"
                  class="min-w-[22px] rounded border border-line/60 bg-panel/60 px-1.5 py-0.5 text-center font-mono text-2xs text-subtle/70"
                  :title="`Also works: ${aliasParts(alias).join(' ')}`"
                >
                  {{ aliasParts(alias).join('') }}
                </kbd>
                <button
                  v-if="store.shortcutIsCustom(row.id)"
                  class="flex h-6 w-6 items-center justify-center rounded text-subtle transition hover:bg-panel-2 hover:text-ink"
                  :title="`Reset “${row.label}” to its default`"
                  @click="store.resetShortcut(row.id)"
                >
                  <RotateCcw class="h-3 w-3" />
                </button>
              </template>
            </dd>
          </div>
        </dl>
      </section>
    </div>

    <section class="mt-6 border-t border-line pt-4">
      <h3 class="panel-heading mb-2">Fixed navigation and gestures</h3>
      <dl class="grid gap-x-8 gap-y-1.5 sm:grid-cols-2">
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Move selection</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">←</kbd>
            <kbd class="shortcut-kbd">→</kbd>
            <kbd class="shortcut-kbd">↑</kbd>
            <kbd class="shortcut-kbd">↓</kbd>
          </dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Extend selection</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">⇧</kbd>
            <span class="text-2xs text-subtle">+</span>
            <kbd class="shortcut-kbd">Arrows</kbd>
          </dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Close viewer, dialog or clear selection</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">Esc</kbd>
          </dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Delete photo in fullscreen</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">Delete</kbd>
            <kbd class="shortcut-kbd">Backspace</kbd>
          </dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Toggle a photo in the selection</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">⌘</kbd>
            <span class="text-2xs text-subtle">+</span>
            <span class="text-2xs text-muted">Click</span>
          </dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-xs text-muted">Select a range / open in filmstrip</dt>
          <dd class="flex shrink-0 items-center gap-1">
            <kbd class="shortcut-kbd">⇧</kbd>
            <span class="text-2xs text-subtle">+</span>
            <span class="text-2xs text-muted">Click</span>
            <span class="px-0.5 text-2xs text-subtle">/</span>
            <span class="text-2xs text-muted">Double-click</span>
          </dd>
        </div>
      </dl>
    </section>

    <template #footer>
      <button class="btn" :disabled="!hasCustomShortcuts" @click="resetAll">
        <RotateCcw class="h-3.5 w-3.5" />
        Reset all
      </button>
      <button class="btn btn-primary" @click="store.closeShortcuts()">Done</button>
    </template>
  </AppDialog>
</template>

<style scoped>
.shortcut-kbd {
  @apply min-w-[22px] rounded border border-line bg-panel px-1.5 py-0.5 text-center font-mono text-2xs text-muted shadow-sm;
}
</style>
