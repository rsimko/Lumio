<script setup lang="ts">
import SettingsRow from '@/components/settings/SettingsRow.vue';

withDefaults(
  defineProps<{
    title: string;
    description?: string;
    modelValue: boolean;
    disabled?: boolean;
  }>(),
  { description: '', disabled: false },
);

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();
</script>

<template>
  <SettingsRow :title="title" :description="description" :disabled="disabled">
    <button
      class="switch"
      :class="modelValue ? 'switch-on' : ''"
      role="switch"
      :aria-checked="modelValue"
      :aria-label="title"
      :disabled="disabled"
      @click="emit('update:modelValue', !modelValue)"
    >
      <span class="switch-knob" />
    </button>
  </SettingsRow>
</template>
