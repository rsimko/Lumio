<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';

const props = withDefaults(
  defineProps<{
    width: number;
    min: number;
    max: number;
    side?: 'left' | 'right';
  }>(),
  { side: 'right' },
);

const emit = defineEmits<{
  resize: [value: number];
  commit: [value: number];
}>();

const dragging = ref(false);
let startX = 0;
let startWidth = 0;
let latest = props.width;

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const delta = props.side === 'left' ? startX - event.clientX : event.clientX - startX;
  latest = Math.min(props.max, Math.max(props.min, Math.round(startWidth + delta)));
  emit('resize', latest);
}

function stop() {
  if (!dragging.value) return;
  dragging.value = false;
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', stop);
  document.body.style.removeProperty('cursor');
  emit('commit', latest);
}

function start(event: PointerEvent) {
  event.preventDefault();
  dragging.value = true;
  startX = event.clientX;
  startWidth = props.width;
  latest = props.width;
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', stop);
  document.body.style.cursor = 'col-resize';
}

onBeforeUnmount(stop);
</script>

<template>
  <div
    class="group absolute inset-y-0 z-20 w-1.5 cursor-col-resize"
    :class="side === 'left' ? 'left-0' : 'right-0'"
    @pointerdown="start"
  >
    <div
      class="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 transition-colors"
      :class="dragging ? 'bg-accent/70' : 'bg-transparent group-hover:bg-accent/50'"
    />
  </div>
</template>
