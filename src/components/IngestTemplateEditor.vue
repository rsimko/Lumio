<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next';
import { computed, nextTick, ref } from 'vue';

import IngestTokenPalette from '@/components/IngestTokenPalette.vue';
import {
  insertIngestToken,
  validateIngestTemplate,
  type IngestTemplateKind,
} from '@/lib/ingest';

const props = defineProps<{
  modelValue: string;
  label: string;
  kind: IngestTemplateKind;
  jobCode?: string;
  customText?: string;
  hint?: string;
  placeholder?: string;
  mono?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const input = ref<HTMLInputElement | null>(null);
const caret = ref(0);
const focused = ref(false);

const problems = computed(() =>
  props.modelValue.trim() === ''
    ? []
    : validateIngestTemplate(props.modelValue, props.kind, {
        jobCode: props.jobCode,
        customText: props.customText,
      }),
);

function rememberCaret(): void {
  const element = input.value;
  if (!element) return;
  caret.value = element.selectionStart ?? element.value.length;
}

function insert(token: string): void {
  const element = input.value;
  const start = focused.value && element ? (element.selectionStart ?? caret.value) : caret.value;
  const end = focused.value && element ? (element.selectionEnd ?? start) : start;
  const result = insertIngestToken(props.modelValue, token, start, end);
  caret.value = result.caret;
  emit('update:modelValue', result.value);
  void nextTick(() => {
    element?.focus();
    element?.setSelectionRange(result.caret, result.caret);
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

function onDragOver(event: DragEvent): void {
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
}
</script>

<template>
  <label class="block">
    <span class="mb-1 flex items-baseline justify-between gap-2">
      <span class="text-2xs font-medium text-ink">{{ label }}</span>
      <span v-if="hint" class="truncate text-[10px] text-subtle">{{ hint }}</span>
    </span>
    <input
      ref="input"
      class="w-full rounded-md border bg-panel px-2 py-1.5 text-xs text-ink outline-none transition placeholder:text-subtle/60 focus:border-accent/70 focus:ring-1 focus:ring-accent/30"
      :class="[
        mono === false ? '' : 'font-mono',
        problems.length ? 'border-danger/60' : 'border-line',
      ]"
      :value="modelValue"
      :placeholder="placeholder"
      spellcheck="false"
      @input="
        emit('update:modelValue', ($event.target as HTMLInputElement).value);
        rememberCaret();
      "
      @focus="
        focused = true;
        rememberCaret();
      "
      @blur="
        focused = false;
        rememberCaret();
      "
      @click="rememberCaret"
      @keyup="rememberCaret"
      @select="rememberCaret"
      @dragover="onDragOver"
      @drop="onDrop"
    />
    <IngestTokenPalette class="mt-1.5" @insert="insert" />
    <ul v-if="problems.length" class="mt-1 space-y-0.5">
      <li
        v-for="problem in problems"
        :key="problem"
        class="flex items-start gap-1 text-[10px] leading-snug text-danger"
      >
        <AlertTriangle class="mt-px h-2.5 w-2.5 shrink-0" />
        {{ problem }}
      </li>
    </ul>
  </label>
</template>
