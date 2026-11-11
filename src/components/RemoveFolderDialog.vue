<script setup lang="ts">
import { AlertTriangle, Check, FolderMinus, Loader2, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';
import type { RootRemovalMode } from '@/types';

const store = useLibraryStore();

const mode = ref<RootRemovalMode>('libraryOnly');
const acknowledged = ref(false);
const localError = ref<string | null>(null);

const targets = computed(() => store.removeFolderTargets);
const count = computed(() => targets.value.length);
const isMultiple = computed(() => count.value > 1);
const hasRoots = computed(() => targets.value.some((target) => target.kind === 'root'));
const isDestructive = computed(() => mode.value === 'moveToTrash');
const canConfirm = computed(() => {
  if (!targets.value.length || store.removingFolder) return false;
  if (!isDestructive.value) return true;
  return acknowledged.value;
});
const error = computed(() => localError.value ?? store.removeFolderError);
const confirmLabel = computed(() => {
  if (isDestructive.value) {
    return isMultiple.value ? `Move ${count.value} folders to Trash` : 'Move folder to Trash';
  }
  return isMultiple.value ? `Remove ${count.value} folders` : 'Remove from library';
});

async function confirm() {
  if (!targets.value.length || !canConfirm.value) return;
  localError.value = null;
  const ok = await store.removeFolders(targets.value, mode.value);
  if (!ok) {
    localError.value = store.removeFolderError ?? 'The folders could not be removed.';
  }
}
</script>

<template>
  <AppDialog
    v-if="count"
    :title="isMultiple ? 'Remove folders' : 'Remove folder'"
    :subtitle="
      isMultiple
        ? 'Choose what happens to these folders and everything inside them.'
        : 'Choose what happens to this folder and its contents.'
    "
    @close="store.closeRemoveFolder()"
  >
    <div class="space-y-4">
      <div class="rounded-lg border border-line bg-panel px-3 py-3">
        <div class="flex items-start gap-3">
          <div class="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-panel-2 text-muted">
            <FolderMinus class="h-4 w-4" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="break-words text-xs font-semibold text-ink">
              {{ isMultiple ? `${count} folders selected` : targets[0]?.name }}
            </p>
            <p v-if="!isMultiple" class="mt-0.5 break-all font-mono text-2xs text-subtle selectable">
              {{ targets[0]?.path }}
            </p>
            <p v-else class="mt-0.5 text-2xs leading-relaxed text-subtle">
              The whole selection is removed together. Nothing changes until you confirm.
            </p>
          </div>
        </div>

        <ul
          v-if="isMultiple"
          class="mt-2 max-h-36 space-y-0.5 overflow-y-auto border-t border-line/70 pt-2"
        >
          <li
            v-for="target in targets"
            :key="target.path"
            class="flex min-w-0 items-baseline gap-2 px-0.5 py-0.5"
          >
            <FolderMinus class="h-3 w-3 shrink-0 translate-y-0.5 text-subtle" />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[11px] font-medium text-ink">{{ target.name }}</span>
              <span class="block truncate font-mono text-[9px] text-subtle" :title="target.path">
                {{ target.path }}
              </span>
            </span>
            <span
              v-if="target.kind === 'root'"
              class="shrink-0 rounded border border-line px-1 text-[8px] uppercase text-muted"
            >
              library
            </span>
          </li>
        </ul>
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
              <template v-if="isMultiple">
                Lumio stops tracking selected library roots and hides selected subfolders. Every
                photo and file stays exactly where it is on disk.
              </template>
              <template v-else-if="hasRoots">
                Lumio stops tracking this folder. Every photo and file stays exactly where it is on
                disk.
              </template>
              <template v-else>
                Lumio hides this folder from the tree. Every photo and file stays exactly where it
                is on disk; add it again to restore it.
              </template>
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
              Delete folders from disk
              <Check v-if="isDestructive" class="ml-auto h-3.5 w-3.5 shrink-0 text-danger" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              <template v-if="isMultiple">
                Moves the selected folders and everything inside them to the system Trash. You can
                restore them from the Trash until you empty it.
              </template>
              <template v-else>
                Moves the entire folder to the system Trash. You can restore it from the Trash
                until you empty it.
              </template>
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
            <template v-if="isMultiple">
              <span class="font-semibold text-danger">
                This moves {{ count }} folders to the Trash.
              </span>
              All photos, subfolders and sidecar files inside them will leave their current
              location. Library roots will be unregistered; parent library folders stay in Lumio.
            </template>
            <template v-else>
              <span class="font-semibold text-danger">
                This moves the whole folder to the Trash.
              </span>
              All photos, subfolders and sidecar files inside
              <span class="font-mono text-2xs">{{ targets[0]?.name }}</span> will leave their
              current location.
              <template v-if="hasRoots">This cannot be undone from Lumio.</template>
              <template v-else>The parent library folder stays in Lumio.</template>
            </template>
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
          <span>
            {{
              isMultiple
                ? 'I understand these folders and everything in them will be moved to the Trash.'
                : 'I understand this folder and everything in it will be moved to the Trash.'
            }}
          </span>
        </label>
      </div>

      <p
        v-if="error"
        class="max-h-28 overflow-y-auto whitespace-pre-line rounded-md border border-danger/30 bg-danger/5 px-3 py-2 text-xs leading-relaxed text-danger"
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
