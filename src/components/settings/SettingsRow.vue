<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    disabled?: boolean;
  }>(),
  { description: '', disabled: false },
);
</script>

<template>
  <div
    class="flex items-center justify-between gap-4 px-3.5 py-3"
    :class="disabled ? 'opacity-55' : ''"
  >
    <div class="min-w-0">
      <p class="text-xs font-medium text-ink">{{ title }}</p>
      <p v-if="description" class="mt-0.5 text-2xs leading-relaxed text-subtle">
        {{ description }}
      </p>
    </div>
    <div class="shrink-0">
      <slot />
    </div>
  </div>
</template>
