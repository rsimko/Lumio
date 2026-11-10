<script setup lang="ts">
import { AlertTriangle, CheckCircle2 } from 'lucide-vue-next';

import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
</script>

<template>
  <AppDialog
    title="Scan warnings"
    :subtitle="
      store.warningsCount
        ? `${store.warningsCount} item${store.warningsCount === 1 ? '' : 's'} could not be read during the last scan.`
        : 'The last scan completed cleanly.'
    "
    wide
    @close="store.closeWarnings()"
  >
    <div v-if="store.warnings.length" class="space-y-2">
      <div
        v-for="(warning, index) in store.warnings"
        :key="`${warning.path}-${index}`"
        class="rounded-md border border-warning/25 bg-warning/5 px-3 py-2"
      >
        <div class="flex items-start gap-2">
          <AlertTriangle class="mt-0.5 h-3.5 w-3.5 shrink-0 text-warning" />
          <div class="min-w-0">
            <p class="break-all font-mono text-2xs text-muted selectable">{{ warning.path }}</p>
            <p class="mt-1 text-xs leading-relaxed text-ink">{{ warning.message }}</p>
          </div>
        </div>
      </div>
      <p class="pt-1 text-2xs leading-relaxed text-subtle">
        Other photos were scanned normally. Warnings usually indicate permission problems,
        unreadable files or folders that disappeared during scanning.
      </p>
    </div>

    <div v-else class="flex flex-col items-center py-6 text-center">
      <CheckCircle2 class="mb-2 h-8 w-8 text-success" />
      <p class="text-sm font-medium text-ink">No warnings</p>
      <p class="mt-1 text-xs text-muted">Every item in this folder was read successfully.</p>
    </div>

    <template #footer>
      <button class="btn" @click="store.closeWarnings()">Close</button>
    </template>
  </AppDialog>
</template>
