<script setup lang="ts">
import { GripVertical } from 'lucide-vue-next';

import { INGEST_TOKEN_GROUPS } from '@/lib/ingest';

defineEmits<{
  insert: [token: string];
}>();

function onDragStart(event: DragEvent, token: string): void {
  if (!event.dataTransfer) return;
  event.dataTransfer.effectAllowed = 'copy';
  event.dataTransfer.setData('text/x-lumio-token', token);
  event.dataTransfer.setData('text/plain', `{${token}}`);
}
</script>

<template>
  <div class="flex flex-wrap items-start gap-x-3 gap-y-1.5">
    <div
      v-for="group in INGEST_TOKEN_GROUPS"
      :key="group.label"
      class="flex flex-wrap items-center gap-1"
    >
      <span class="mr-0.5 text-[10px] font-medium uppercase tracking-wide text-subtle">
        {{ group.label }}
      </span>
      <button
        v-for="token in group.tokens"
        :key="token.token"
        type="button"
        draggable="true"
        class="group inline-flex h-5 items-center gap-0.5 rounded border border-line bg-panel px-1.5 font-mono text-[10px] text-muted transition hover:border-line-strong hover:bg-panel-2 hover:text-ink active:bg-line"
        :title="`${token.description} — click or drag into a template`"
        @click="$emit('insert', token.token)"
        @dragstart="onDragStart($event, token.token)"
      >
        <GripVertical class="h-2.5 w-2.5 text-subtle/60 group-hover:text-subtle" />
        {{ '{' + token.token + '}' }}
      </button>
    </div>
  </div>
</template>
