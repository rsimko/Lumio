<script setup lang="ts">
import { AlertTriangle, CheckCircle2, Info, X } from 'lucide-vue-next';

import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
</script>

<template>
  <Teleport to="body">
    <div class="pointer-events-none fixed inset-x-0 bottom-10 z-[60] flex flex-col items-center gap-2 px-6">
      <TransitionGroup name="toast">
        <div
          v-for="toast in store.toasts"
          :key="toast.id"
          class="pointer-events-auto flex max-w-md items-start gap-2.5 rounded-lg border border-line bg-elevated px-3 py-2.5 shadow-float"
          :class="{
            'border-danger/30': toast.kind === 'error',
            'border-success/30': toast.kind === 'success',
          }"
          role="status"
        >
          <AlertTriangle
            v-if="toast.kind === 'error'"
            class="mt-px h-4 w-4 shrink-0 text-danger"
          />
          <CheckCircle2
            v-else-if="toast.kind === 'success'"
            class="mt-px h-4 w-4 shrink-0 text-success"
          />
          <Info v-else class="mt-px h-4 w-4 shrink-0 text-muted" />
          <p class="min-w-0 flex-1 text-xs leading-relaxed text-ink">{{ toast.message }}</p>
          <button
            class="mt-px text-subtle transition hover:text-ink"
            title="Dismiss"
            @click="store.dismissToast(toast.id)"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>
