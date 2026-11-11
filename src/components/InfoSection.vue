<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next';
import { ref } from 'vue';

const props = withDefaults(
  defineProps<{
    title: string;
    defaultOpen?: boolean;
  }>(),
  { defaultOpen: true },
);

const open = ref(props.defaultOpen);
</script>

<template>
  <section class="border-t border-line" :aria-label="title">
    <div class="flex items-center gap-2 px-3 py-1.5">
      <h4 class="min-w-0 flex-1">
        <button
          class="group flex w-full min-w-0 items-center gap-1.5 rounded py-1 text-left transition"
          :aria-expanded="open"
          @click="open = !open"
        >
          <ChevronDown
            class="h-3 w-3 shrink-0 text-subtle transition-transform group-hover:text-ink"
            :class="open ? '' : '-rotate-90'"
          />
          <span class="panel-heading transition group-hover:text-ink">{{ title }}</span>
        </button>
      </h4>
      <slot name="actions" />
    </div>
    <div v-show="open" class="px-3 pb-3">
      <slot />
    </div>
  </section>
</template>
