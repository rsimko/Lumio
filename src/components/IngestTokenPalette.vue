<script setup lang="ts">
import { GripVertical } from 'lucide-vue-next';

import { INGEST_TOKEN_GROUPS, ingestTokenSnippet, type IngestToken } from '@/lib/ingest';

const props = withDefaults(
  defineProps<{
    /** Tokens that do not make sense for the focused field (for example '/' in a filename). */
    disabledTokens?: string[];
  }>(),
  { disabledTokens: () => [] },
);

defineEmits<{
  insert: [token: string];
}>();

function isDisabled(token: string): boolean {
  return props.disabledTokens.includes(token);
}

function tokenLabel(token: IngestToken): string {
  return token.literal ? token.token : `{${token.token}}`;
}

function onDragStart(event: DragEvent, token: string): void {
  if (isDisabled(token)) {
    event.preventDefault();
    return;
  }
  if (!event.dataTransfer) return;
  event.dataTransfer.effectAllowed = 'copy';
  event.dataTransfer.setData('text/x-lumio-token', token);
  event.dataTransfer.setData('text/plain', ingestTokenSnippet(token));
}
</script>

<template>
  <div class="flex flex-wrap items-start gap-x-3 gap-y-2">
    <div
      v-for="group in INGEST_TOKEN_GROUPS"
      :key="group.label"
      class="flex flex-wrap items-center gap-1.5"
    >
      <span class="mr-0.5 text-[9px] font-medium uppercase tracking-wide text-subtle">
        {{ group.label }}
      </span>
      <button
        v-for="token in group.tokens"
        :key="token.token"
        type="button"
        class="group inline-flex h-5 min-w-[22px] items-center justify-center gap-0.5 rounded border border-line-strong/70 bg-elevated px-1.5 font-mono text-[10px] text-muted shadow-sm transition hover:border-accent/40 hover:bg-panel-2 hover:text-ink disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:border-line-strong/70 disabled:hover:bg-elevated disabled:hover:text-muted"
        :disabled="isDisabled(token.token)"
        :draggable="!isDisabled(token.token)"
        :title="
          isDisabled(token.token)
            ? `${token.description} — unavailable in this field`
            : `${token.description} — click or drag into a template`
        "
        @click="$emit('insert', token.token)"
        @dragstart="onDragStart($event, token.token)"
      >
        <GripVertical class="h-2.5 w-2.5 text-subtle/50 group-hover:text-subtle" />
        {{ tokenLabel(token) }}
      </button>
    </div>
  </div>
</template>
