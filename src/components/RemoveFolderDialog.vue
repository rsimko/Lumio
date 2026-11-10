<script setup lang="ts">
import {
  AlertTriangle,
  Check,
  FolderMinus,
  Loader2,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';
import type { RootRemovalMode } from '@/types';

const store = useLibraryStore();

const mode = ref<RootRemovalMode>('libraryOnly');
const acknowledged = ref(false);
const typedName = ref('');
const localError = ref<string | null>(null);

const target = computed(() => store.removeFolderTarget);
const isDestructive = computed(() => mode.value === 'moveToTrash');
const nameMatches = computed(
  () => !!target.value && typedName.value.trim() === target.value.name,
);
const canConfirm = computed(() => {
  if (!target.value || store.removingFolder) return false;
  if (!isDestructive.value) return true;
  return acknowledged.value && nameMatches.value;
});
const error = computed(() => localError.value ?? store.removeFolderError);

const confirmLabel = computed(() => {
  if (isDestructive.value) return 'Move folder to Trash';
  return 'Remove from library';
});

async function confirm() {
  if (!target.value || !canConfirm.value) return;
  localError.value = null;
  const ok = await store.removeRoot(target.value.id, mode.value);
  if (!ok) {
    localError.value = store.removeFolderError ?? 'The folder could not be removed.';
  }
}
</script>

<template>
  <AppDialog
    v-if="target"
    title="Remove folder"
    subtitle="Choose what happens to this folder and its contents."
    @close="store.closeRemoveFolder()"
  >
    <div class="space-y-4">
      <div class="flex items-start gap-3 rounded-lg border border-line bg-panel px-3 py-3">
        <div class="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-panel-2 text-muted">
          <FolderMinus class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <p class="break-words text-xs font-semibold text-ink">{{ target.name }}</p>
          <p class="mt-0.5 break-all font-mono text-2xs text-subtle selectable">
            {{ target.path }}
          </p>
        </div>
      </div>

      <div class="space-y-2">
        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            !isDestructive
              ? 'border-accent bg-accent-soft/60'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="!isDestructive"
          @click="mode = 'libraryOnly'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="!isDestructive ? 'bg-accent/15 text-accent' : 'bg-panel-2 text-muted'"
          >
            <FolderMinus class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Remove from library only
              <span class="badge bg-accent/10 text-accent">Recommended</span>
              <Check v-if="!isDestructive" class="ml-auto h-3.5 w-3.5 shrink-0 text-accent" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Lumio stops tracking this folder. Every photo and file stays exactly where it is on
              disk.
            </span>
          </span>
        </button>

        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            isDestructive
              ? 'border-danger/50 bg-danger/5'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="isDestructive"
          @click="mode = 'moveToTrash'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="isDestructive ? 'bg-danger/15 text-danger' : 'bg-panel-2 text-muted'"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Delete folder from disk
              <Check v-if="isDestructive" class="ml-auto h-3.5 w-3.5 shrink-0 text-danger" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Moves the entire folder to the system Trash. You can restore it from the Trash until
              you empty it.
            </span>
          </span>
        </button>
      </div>

      <div
        v-if="isDestructive"
        class="space-y-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-3"
      >
        <div class="flex items-start gap-2">
          <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-danger" />
          <p class="text-xs leading-relaxed text-ink">
            <span class="font-semibold text-danger">This moves the whole folder to the Trash.</span>
            All photos, subfolders and sidecar files inside
            <span class="font-mono text-2xs">{{ target.name }}</span> will leave their current
            location. This cannot be undone from Lumio.
          </p>
        </div>

        <label
          class="flex cursor-pointer items-start gap-2 rounded-md border border-danger/20 bg-elevated/70 px-2.5 py-2 text-xs leading-relaxed text-ink"
        >
          <input
            v-model="acknowledged"
            type="checkbox"
            class="mt-0.5 h-3.5 w-3.5 shrink-0 accent-danger"
          />
          <span>I understand this folder and everything in it will be moved to the Trash.</span>
        </label>

        <label class="block">
          <span class="mb-1 block text-2xs text-muted">
            Type <span class="font-semibold text-ink">{{ target.name }}</span> to confirm
          </span>
          <input
            v-model="typedName"
            type="text"
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            class="h-8 w-full rounded-md border bg-panel px-2.5 text-xs text-ink outline-none transition placeholder:text-subtle focus:ring-2"
            :class="
              typedName && !nameMatches
                ? 'border-danger/60 focus:border-danger/60 focus:ring-danger/20'
                : 'border-line-strong focus:border-danger/60 focus:ring-danger/20'
            "
            :placeholder="target.name"
            @keydown.enter.prevent="confirm"
          />
        </label>
      </div>

      <p
        v-if="error"
        class="rounded-md border border-danger/30 bg-danger/5 px-3 py-2 text-xs leading-relaxed text-danger"
      >
        {{ error }}
      </p>
    </div>

    <template #footer>
      <button class="btn" :disabled="store.removingFolder" @click="store.closeRemoveFolder()">
        Cancel
      </button>
      <button
        :class="isDestructive ? 'btn-danger' : 'btn-primary'"
        :disabled="!canConfirm"
        @click="confirm"
      >
        <Loader2 v-if="store.removingFolder" class="h-3.5 w-3.5 animate-spin" />
        <Trash2 v-else-if="isDestructive" class="h-3.5 w-3.5" />
        <FolderMinus v-else class="h-3.5 w-3.5" />
        {{ confirmLabel }}
      </button>
    </template>
  </AppDialog>
</template>
