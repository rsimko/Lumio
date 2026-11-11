<script setup lang="ts">
import { AlertTriangle, Check, Plus, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { formatCount, pluralize } from '@/lib/format';
import type { IngestEntity } from '@/types';

const props = defineProps<{
  entities: IngestEntity[];
}>();

const emit = defineEmits<{
  close: [];
  resolve: [resolution: 'keepBoth' | 'replace'];
}>();

const mode = ref<'keepBoth' | 'replace'>('replace');
const photoCount = computed(() => props.entities.length);
const existingPaths = computed(() => [
  ...new Set(props.entities.flatMap((entity) => entity.conflict?.existing ?? [])),
]);
const targetName = computed(() => {
  if (photoCount.value === 1) return props.entities[0]?.displayName ?? 'This photo';
  return `${formatCount(photoCount.value)} photos`;
});
</script>

<template>
  <AppDialog
    title="Destination collision"
    subtitle="Existing files are in the way. Choose whether to keep both copies or replace what is already there."
    @close="emit('close')"
  >
    <div class="space-y-4">
      <div class="flex items-start gap-3 rounded-lg border border-warning/30 bg-warning/10 px-3 py-3">
        <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-warning" />
        <div class="min-w-0 flex-1">
          <p class="break-words text-xs font-semibold text-ink">{{ targetName }}</p>
          <p class="mt-0.5 text-2xs leading-relaxed text-muted">
            {{ formatCount(existingPaths.length) }}
            {{ pluralize(existingPaths.length, 'existing file is', 'existing files are') }}
            already in the destination.
          </p>
          <details class="mt-1.5">
            <summary class="cursor-pointer text-2xs text-warning">Show existing paths</summary>
            <ul class="mt-1 max-h-28 space-y-0.5 overflow-y-auto">
              <li
                v-for="path in existingPaths"
                :key="path"
                class="truncate font-mono text-2xs text-muted selectable"
                :title="path"
              >
                {{ path }}
              </li>
            </ul>
          </details>
        </div>
      </div>

      <div class="space-y-2">
        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            mode === 'keepBoth'
              ? 'border-accent bg-accent-soft/60'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="mode === 'keepBoth'"
          @click="mode = 'keepBoth'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="mode === 'keepBoth' ? 'bg-accent/15 text-accent' : 'bg-panel-2 text-muted'"
          >
            <Plus class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Rename and keep both
              <Check v-if="mode === 'keepBoth'" class="ml-auto h-3.5 w-3.5 shrink-0 text-accent" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Appends _2, _3, … to the incoming photo group. Existing destination files stay
              exactly as they are.
            </span>
          </span>
        </button>

        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            mode === 'replace'
              ? 'border-danger/50 bg-danger/5'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="mode === 'replace'"
          @click="mode = 'replace'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="mode === 'replace' ? 'bg-danger/15 text-danger' : 'bg-panel-2 text-muted'"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Replace existing files
              <Check v-if="mode === 'replace'" class="ml-auto h-3.5 w-3.5 shrink-0 text-danger" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Existing files are backed up while the copy is committed and restored if the ingest
              fails.
            </span>
          </span>
        </button>
      </div>
    </div>

    <template #footer>
      <button class="btn" @click="emit('close')">Cancel</button>
      <button
        :class="mode === 'replace' ? 'btn-danger' : 'btn-primary'"
        @click="emit('resolve', mode)"
      >
        <Trash2 v-if="mode === 'replace'" class="h-3.5 w-3.5" />
        <Plus v-else class="h-3.5 w-3.5" />
        {{ mode === 'replace' ? 'Replace existing files' : 'Rename and keep both' }}
      </button>
    </template>
  </AppDialog>
</template>
