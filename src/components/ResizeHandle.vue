<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';

const props = withDefaults(
  defineProps<{
    width: number;
    min: number;
    max: number;
    /** Which edge of the parent the handle sits on. */
    side?: 'left' | 'right' | 'top';
  }>(),
  { side: 'right' },
);

const emit = defineEmits<{
  resize: [value: number];
  commit: [value: number];
}>();

const dragging = ref(false);
let startX = 0;
let startY = 0;
let startWidth = 0;
let latest = props.width;

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const delta =
    props.side === 'left'
      ? startX - event.clientX
      : props.side === 'top'
        ? startY - event.clientY
        : event.clientX - startX;
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
  startY = event.clientY;
  startWidth = props.width;
  latest = props.width;
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', stop);
  document.body.style.cursor = props.side === 'top' ? 'row-resize' : 'col-resize';
}

onBeforeUnmount(stop);
</script>

<template>
  <div
    class="group absolute z-20"
    :class="
      side === 'left'
        ? 'inset-y-0 left-0 w-1.5 cursor-col-resize'
        : side === 'top'
          ? 'inset-x-0 top-0 h-1.5 cursor-row-resize'
          : 'inset-y-0 right-0 w-1.5 cursor-col-resize'
    "
    @pointerdown="start"
  >
    <div
      class="absolute transition-colors"
      :class="[
        side === 'top'
          ? 'inset-x-0 top-1/2 h-px -translate-y-1/2'
          : 'inset-y-0 left-1/2 w-px -translate-x-1/2',
        dragging ? 'bg-accent/70' : 'bg-transparent group-hover:bg-accent/50',
      ]"
    />
  </div>
</template>
