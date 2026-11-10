<script setup lang="ts">
import { Loader2 } from 'lucide-vue-next';
import { computed } from 'vue';

import type { HistogramData } from '@/types';

const props = defineProps<{
  data: HistogramData | null;
  loading: boolean;
  error: string | null;
}>();

const WIDTH = 256;
const HEIGHT = 96;

const peak = computed(() => {
  if (!props.data) return 1;
  let max = 0;
  for (const bins of [props.data.red, props.data.green, props.data.blue, props.data.luma]) {
    for (const count of bins) {
      if (count > max) max = count;
    }
  }
  return max || 1;
});

function curve(bins: readonly number[]): string {
  if (!bins.length) return '';
  const last = bins.length - 1;
  let path = '';
  bins.forEach((count, index) => {
    const x = last === 0 ? 0 : (index / last) * WIDTH;
    // A square-root scale keeps shadow and midtone detail readable when one
    // clipped bin holds most of the pixels.
    const y = HEIGHT - Math.sqrt(count / peak.value) * HEIGHT;
    path += `${index === 0 ? 'M' : 'L'} ${x.toFixed(2)} ${y.toFixed(2)} `;
  });
  return path.trim();
}

function area(bins: readonly number[]): string {
  const line = curve(bins);
  if (!line) return '';
  return `${line} L ${WIDTH} ${HEIGHT} L 0 ${HEIGHT} Z`;
}

const redPath = computed(() => (props.data ? area(props.data.red) : ''));
const greenPath = computed(() => (props.data ? area(props.data.green) : ''));
const bluePath = computed(() => (props.data ? area(props.data.blue) : ''));
const lumaPath = computed(() => (props.data ? curve(props.data.luma) : ''));
</script>

<template>
  <div class="relative h-full w-full overflow-hidden bg-[#0b0c0e]">
    <div
      v-if="loading"
      class="absolute inset-0 flex flex-col items-center justify-center gap-1.5 text-subtle"
    >
      <Loader2 class="h-4 w-4 animate-spin" />
      <span class="text-2xs">Analyzing preview…</span>
    </div>
    <div
      v-else-if="error"
      class="absolute inset-0 flex items-center justify-center px-6 text-center"
    >
      <p class="text-2xs leading-relaxed text-danger">{{ error }}</p>
    </div>
    <div
      v-else-if="!data"
      class="absolute inset-0 flex items-center justify-center text-2xs text-subtle"
    >
      No histogram data
    </div>
    <svg
      v-else
      class="h-full w-full"
      viewBox="0 0 256 96"
      preserveAspectRatio="none"
      role="img"
      :aria-label="`RGB histogram of ${data.pixels.toLocaleString()} preview pixels`"
    >
      <g stroke="rgb(255 255 255 / 0.08)" stroke-width="1" vector-effect="non-scaling-stroke">
        <line x1="64" y1="0" x2="64" y2="96" />
        <line x1="128" y1="0" x2="128" y2="96" />
        <line x1="192" y1="0" x2="192" y2="96" />
        <line x1="0" y1="32" x2="256" y2="32" />
        <line x1="0" y1="64" x2="256" y2="64" />
      </g>
      <g style="mix-blend-mode: screen">
        <path :d="redPath" fill="rgb(239 83 80)" fill-opacity="0.5" />
        <path :d="greenPath" fill="rgb(76 175 110)" fill-opacity="0.5" />
        <path :d="bluePath" fill="rgb(64 128 230)" fill-opacity="0.5" />
      </g>
      <path
        :d="lumaPath"
        fill="none"
        stroke="rgb(255 255 255 / 0.62)"
        stroke-width="1.1"
        vector-effect="non-scaling-stroke"
      />
    </svg>
  </div>
</template>
