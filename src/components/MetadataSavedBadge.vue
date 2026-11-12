<script setup lang="ts">
import { Check } from 'lucide-vue-next';
import { onUnmounted, ref, watch } from 'vue';

import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const visible = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;

watch(
  () => store.metadataSavedAt,
  (value) => {
    if (!value) return;
    visible.value = true;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      visible.value = false;
    }, 1600);
  },
);

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <Transition
    enter-active-class="transition duration-150 ease-out"
    enter-from-class="translate-y-0.5 opacity-0"
    leave-active-class="transition duration-200 ease-in"
    leave-to-class="opacity-0"
  >
    <span
      v-if="visible"
      class="inline-flex items-center gap-1 text-2xs font-medium text-success"
      role="status"
      aria-live="polite"
    >
      <Check class="h-3 w-3" />
      Saved
    </span>
  </Transition>
</template>
