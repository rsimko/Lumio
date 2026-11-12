<script setup lang="ts">
import { computed, ref, watch } from 'vue';

const props = withDefaults(
  defineProps<{
    label: string;
    value: string;
    mixed?: boolean;
    placeholder?: string;
    multiline?: boolean;
    maxlength?: number;
    inputmode?: 'text' | 'decimal';
    hint?: string;
    invalid?: boolean;
  }>(),
  {
    mixed: false,
    placeholder: '',
    multiline: false,
    maxlength: undefined,
    inputmode: 'text',
    hint: '',
    invalid: false,
  },
);

const emit = defineEmits<{
  commit: [value: string];
}>();

const draft = ref(props.mixed ? '' : props.value);
const focused = ref(false);
const dirty = ref(false);
const displayValue = computed(() => (props.mixed ? '' : props.value));

watch(displayValue, (value) => {
  if (!focused.value) draft.value = value;
});

function commit(): void {
  const next = draft.value.trim();
  const changed = dirty.value || next !== displayValue.value;
  dirty.value = false;
  if (!changed) return;
  emit('commit', next);
}

function onInput(): void {
  dirty.value = true;
}

function onBlur(): void {
  focused.value = false;
  commit();
}

function onFocus(): void {
  focused.value = true;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault();
    draft.value = displayValue.value;
    dirty.value = false;
    (event.target as HTMLInputElement | HTMLTextAreaElement).blur();
  } else if (event.key === 'Enter' && !props.multiline) {
    event.preventDefault();
    (event.target as HTMLInputElement).blur();
  } else if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
    event.preventDefault();
    (event.target as HTMLTextAreaElement).blur();
  }
}
</script>

<template>
  <label class="block">
    <span class="mb-1 flex items-baseline justify-between gap-2">
      <span class="text-2xs font-medium text-ink">{{ label }}</span>
      <span v-if="hint" class="truncate text-[10px] text-subtle">{{ hint }}</span>
    </span>
    <textarea
      v-if="multiline"
      v-model="draft"
      rows="2"
      class="w-full resize-y rounded-md border bg-panel px-2 py-1.5 text-xs leading-relaxed text-ink outline-none transition placeholder:text-subtle/60 focus:border-accent/70 focus:ring-1 focus:ring-accent/30"
      :class="invalid ? 'border-danger/60' : 'border-line'"
      :placeholder="mixed ? 'Mixed values' : placeholder"
      :maxlength="maxlength"
      :aria-label="label"
      spellcheck="false"
      @input="onInput"
      @focus="onFocus"
      @blur="onBlur"
      @keydown="onKeydown"
    />
    <input
      v-else
      v-model="draft"
      class="w-full rounded-md border bg-panel px-2 py-1.5 text-xs text-ink outline-none transition placeholder:text-subtle/60 focus:border-accent/70 focus:ring-1 focus:ring-accent/30"
      :class="invalid ? 'border-danger/60' : 'border-line'"
      :placeholder="mixed ? 'Mixed values' : placeholder"
      :maxlength="maxlength"
      :inputmode="inputmode"
      :aria-label="label"
      spellcheck="false"
      @input="onInput"
      @focus="onFocus"
      @blur="onBlur"
      @keydown="onKeydown"
    />
  </label>
</template>
