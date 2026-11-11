<script setup lang="ts">
import { Plus, X } from 'lucide-vue-next';
import { nextTick, ref } from 'vue';

import IngestTokenPalette from '@/components/IngestTokenPalette.vue';
import { insertIngestToken } from '@/lib/ingest';

const props = defineProps<{
  modelValue: string[];
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
}>();

const input = ref<HTMLInputElement | null>(null);
const value = ref('');
const caret = ref(0);

function commit(): void {
  const keyword = value.value.trim();
  if (!keyword) return;
  if (
    props.modelValue.some((existing) => existing.toLowerCase() === keyword.toLowerCase())
  ) {
    value.value = '';
    return;
  }
  emit('update:modelValue', [...props.modelValue, keyword]);
  value.value = '';
  caret.value = 0;
}

function remove(index: number): void {
  emit(
    'update:modelValue',
    props.modelValue.filter((_, current) => current !== index),
  );
}

function insert(token: string): void {
  const start = input.value?.selectionStart ?? caret.value;
  const end = input.value?.selectionEnd ?? start;
  const result = insertIngestToken(value.value, token, start, end);
  value.value = result.value;
  caret.value = result.caret;
  void nextTick(() => {
    input.value?.focus();
    input.value?.setSelectionRange(result.caret, result.caret);
  });
}

function onDrop(event: DragEvent): void {
  const token =
    event.dataTransfer?.getData('text/x-lumio-token') ||
    event.dataTransfer?.getData('text/plain').replace(/[{}]/g, '');
  if (!token) return;
  event.preventDefault();
  insert(token);
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Enter') {
    event.preventDefault();
    commit();
  } else if (event.key === 'Backspace' && !value.value && props.modelValue.length) {
    remove(props.modelValue.length - 1);
  }
}
</script>

<template>
  <div>
    <div v-if="modelValue.length" class="mb-1.5 flex flex-wrap gap-1">
      <span
        v-for="(keyword, index) in modelValue"
        :key="`${keyword}-${index}`"
        class="inline-flex h-5 items-center gap-1 rounded-full border border-line bg-panel px-2 font-mono text-[10px] text-muted"
      >
        {{ keyword }}
        <button
          type="button"
          class="text-subtle transition hover:text-danger"
          :title="`Remove keyword ${keyword}`"
          @click="remove(index)"
        >
          <X class="h-2.5 w-2.5" />
        </button>
      </span>
    </div>
    <div class="flex gap-1">
      <input
        ref="input"
        v-model="value"
        class="min-w-0 flex-1 rounded-md border border-line bg-panel px-2 py-1.5 font-mono text-xs text-ink outline-none transition placeholder:font-sans placeholder:text-subtle/60 focus:border-accent/70 focus:ring-1 focus:ring-accent/30"
        placeholder="Type a keyword, or use a token"
        spellcheck="false"
        @keydown="onKeydown"
        @input="caret = ($event.target as HTMLInputElement).selectionStart ?? 0"
        @dragover.prevent
        @drop="onDrop"
      />
      <button type="button" class="btn" :disabled="!value.trim()" @click="commit">
        <Plus class="h-3 w-3" />
        Add
      </button>
    </div>
    <IngestTokenPalette class="mt-1.5" @insert="insert" />
  </div>
</template>
