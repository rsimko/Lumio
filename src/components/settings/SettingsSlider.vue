<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    modelValue: number;
    min: number;
    max: number;
    step?: number;
    valueLabel?: string;
    disabled?: boolean;
  }>(),
  {
    description: '',
    step: 1,
    valueLabel: '',
    disabled: false,
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: number];
}>();

function onInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value)) emit('update:modelValue', value);
}
</script>

<template>
  <div class="px-3.5 py-3" :class="disabled ? 'opacity-55' : ''">
    <div class="flex items-center justify-between gap-4">
      <div>
        <p class="text-xs font-medium text-ink">{{ title }}</p>
        <p v-if="description" class="mt-0.5 text-2xs leading-relaxed text-subtle">
          {{ description }}
        </p>
      </div>
      <span class="shrink-0 text-2xs tabular-nums text-muted">
        {{ valueLabel || modelValue }}
      </span>
    </div>
    <input
      type="range"
      class="mt-2 w-full cursor-pointer disabled:cursor-default"
      style="accent-color: rgb(var(--c-accent))"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      :disabled="disabled"
      :aria-label="title"
      @input="onInput"
    />
  </div>
</template>
