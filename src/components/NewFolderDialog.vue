<script setup lang="ts">
import { AlertCircle, FolderPlus, Loader2 } from 'lucide-vue-next';
import { computed, nextTick, onMounted, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const name = ref('');
const localError = ref<string | null>(null);
const input = ref<HTMLInputElement | null>(null);

const target = computed(() => store.newFolderTarget);
const error = computed(() => localError.value ?? store.newFolderError);
const canCreate = computed(() => !store.creatingFolder && name.value.trim().length > 0);

function clearErrors() {
  localError.value = null;
  store.clearNewFolderError();
}

async function submit() {
  if (!canCreate.value) return;
  const trimmed = name.value.trim();
  if (!trimmed) {
    localError.value = 'Enter a folder name.';
    return;
  }
  localError.value = null;
  await store.createFolder(trimmed);
}

onMounted(() => {
  void nextTick(() => input.value?.focus());
});
</script>

<template>
  <AppDialog
    v-if="target"
    title="New folder"
    subtitle="Create a folder inside the selected folder."
    @close="store.closeNewFolder()"
  >
    <form class="space-y-4" @submit.prevent="submit" @keydown.esc.stop.prevent="store.closeNewFolder()">
      <div class="flex items-start gap-3 rounded-lg border border-line bg-panel px-3 py-3">
        <div class="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-panel-2 text-muted">
          <FolderPlus class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <p class="break-words text-xs font-semibold text-ink">Inside {{ target.name }}</p>
          <p class="mt-0.5 break-all font-mono text-2xs text-subtle selectable">
            {{ target.path }}
          </p>
        </div>
      </div>

      <label class="block">
        <span class="mb-1.5 block text-2xs font-semibold uppercase tracking-wide text-muted">
          Folder name
        </span>
        <input
          ref="input"
          v-model="name"
          type="text"
          class="h-8 w-full rounded-md border border-line bg-elevated px-3 text-xs text-ink outline-none transition placeholder:text-subtle focus:border-accent focus:ring-2 focus:ring-accent/25 disabled:opacity-50"
          placeholder="New Folder"
          autocomplete="off"
          spellcheck="false"
          :disabled="store.creatingFolder"
          @input="clearErrors"
        />
      </label>

      <p v-if="error" class="flex items-start gap-1.5 text-2xs leading-relaxed text-danger">
        <AlertCircle class="mt-0.5 h-3 w-3 shrink-0" />
        <span class="min-w-0 break-words">{{ error }}</span>
      </p>

      <p v-else class="text-2xs leading-relaxed text-subtle">
        Folders starting with a dot and application or photo packages cannot be created.
      </p>
    </form>

    <template #footer>
      <button
        type="button"
        class="btn-ghost"
        :disabled="store.creatingFolder"
        @click="store.closeNewFolder()"
      >
        Cancel
      </button>
      <button type="button" class="btn-primary" :disabled="!canCreate" @click="submit">
        <Loader2 v-if="store.creatingFolder" class="h-3.5 w-3.5 animate-spin" />
        <FolderPlus v-else class="h-3.5 w-3.5" />
        {{ store.creatingFolder ? 'Creating…' : 'Create Folder' }}
      </button>
    </template>
  </AppDialog>
</template>
