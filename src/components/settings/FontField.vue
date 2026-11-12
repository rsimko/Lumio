<script setup lang="ts">
import { Check } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import {
  MONO_FONT_PRESETS,
  UI_FONT_PRESETS,
  fontChoiceLabel,
  fontFamilyStack,
  resolveFontChoice,
  type FontKind,
} from '@/lib/fonts';

const props = withDefaults(
  defineProps<{
    label: string;
    description: string;
    modelValue: string;
    kind: FontKind;
    fonts: string[];
    loading?: boolean;
    error?: string | null;
  }>(),
  { loading: false, error: null },
);

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const draft = ref(fontChoiceLabel(props.modelValue, props.kind));
const listId = computed(() => `installed-fonts-${props.kind}`);
const presets = computed(() => (props.kind === 'ui' ? UI_FONT_PRESETS : MONO_FONT_PRESETS));

const availableFonts = computed(() => {
  const presetLabels = new Set(presets.value.map((preset) => preset.label.toLowerCase()));
  return props.fonts.filter((family) => !presetLabels.has(family.toLowerCase()));
});

const previewStack = computed(() => {
  const resolved = resolveFontChoice(draft.value, props.kind) ?? props.modelValue;
  return fontFamilyStack(resolved, props.kind);
});

watch(
  () => [props.modelValue, props.kind] as const,
  () => {
    draft.value = fontChoiceLabel(props.modelValue, props.kind);
  },
);

function commit() {
  const resolved = resolveFontChoice(draft.value, props.kind);
  if (!resolved) {
    draft.value = fontChoiceLabel(props.modelValue, props.kind);
    return;
  }
  draft.value = fontChoiceLabel(resolved, props.kind);
  if (resolved !== props.modelValue) emit('update:modelValue', resolved);
}

function applyPreset(value: string) {
  draft.value = fontChoiceLabel(value, props.kind);
  if (value !== props.modelValue) emit('update:modelValue', value);
}
</script>

<template>
  <div class="px-3.5 py-3">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <p class="text-xs font-medium text-ink">{{ label }}</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">{{ description }}</p>
      </div>
      <span v-if="loading" class="shrink-0 text-2xs text-subtle">Reading fonts…</span>
      <span v-else class="shrink-0 text-2xs tabular-nums text-subtle">
        {{ fonts.length }} installed
      </span>
    </div>

    <input
      v-model="draft"
      :list="listId"
      class="mt-2 h-8 w-full rounded-md border border-line bg-panel px-2 text-xs text-ink outline-none transition focus:border-accent/70 focus:ring-1 focus:ring-accent/30"
      :style="{ fontFamily: previewStack }"
      :aria-label="label"
      spellcheck="false"
      autocomplete="off"
      placeholder="Font family"
      @change="commit"
      @blur="commit"
      @keydown.enter.prevent="commit"
      @keydown.escape="draft = fontChoiceLabel(modelValue, kind)"
    />
    <datalist :id="listId">
      <option v-for="preset in presets" :key="preset.value" :value="preset.label">
        {{ preset.description }}
      </option>
      <option v-for="family in availableFonts" :key="family" :value="family"></option>
    </datalist>

    <div class="mt-1.5 flex flex-wrap items-center gap-1">
      <button
        v-for="preset in presets"
        :key="preset.value"
        class="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-2xs transition"
        :class="
          modelValue === preset.value
            ? 'bg-accent-soft font-medium text-accent'
            : 'text-subtle hover:bg-panel-2 hover:text-ink'
        "
        :aria-pressed="modelValue === preset.value"
        :title="`${preset.label} · ${preset.description}`"
        @click="applyPreset(preset.value)"
      >
        <Check v-if="modelValue === preset.value" class="h-2.5 w-2.5" />
        {{ preset.label }}
      </button>
      <span v-if="error" class="ml-auto text-2xs text-warning">
        Installed fonts unavailable — type a family name.
      </span>
    </div>
  </div>
</template>
