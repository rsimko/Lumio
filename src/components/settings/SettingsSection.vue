<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    description?: string;
  }>(),
  { description: '' },
);
</script>

<template>
  <section>
    <header class="mb-2 flex items-start justify-between gap-3">
      <div class="min-w-0">
        <h3 class="text-xs font-semibold text-ink">{{ title }}</h3>
        <p v-if="description" class="mt-0.5 text-2xs leading-relaxed text-subtle">
          {{ description }}
        </p>
      </div>
      <div v-if="$slots.actions" class="shrink-0">
        <slot name="actions" />
      </div>
    </header>
    <div class="overflow-hidden rounded-lg border border-line bg-panel">
      <slot />
    </div>
    <slot name="footer" />
  </section>
</template>
