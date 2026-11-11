<script setup lang="ts">
import { X } from 'lucide-vue-next';

const props = defineProps<{
  title: string;
  subtitle?: string;
  wide?: boolean;
  full?: boolean;
  flush?: boolean;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog" appear>
      <div
        class="fixed inset-0 z-40 flex items-center justify-center bg-black/45 backdrop-blur-[2px]"
        :class="props.full ? 'p-3 sm:p-5' : 'p-6'"
        @pointerdown.self="$emit('close')"
      >
        <div
          class="flex w-full flex-col overflow-hidden rounded-xl border border-line bg-elevated shadow-float"
          :class="[
            props.full
              ? 'h-[94vh] max-w-[min(1440px,96vw)]'
              : 'max-h-[86vh]',
            props.wide ? 'max-w-2xl' : 'max-w-lg',
          ]"
          role="dialog"
          aria-modal="true"
          :aria-label="title"
        >
          <header class="flex items-start justify-between gap-4 border-b border-line px-4 py-3">
            <div>
              <h2 class="text-sm font-semibold text-ink">{{ title }}</h2>
              <p v-if="subtitle" class="mt-0.5 text-xs text-muted">{{ subtitle }}</p>
            </div>
            <button class="btn-icon-sm -mr-1 -mt-0.5" title="Close" @click="$emit('close')">
              <X class="h-3.5 w-3.5" />
            </button>
          </header>
          <div
            v-if="flush"
            class="flex min-h-0 flex-1 flex-col overflow-hidden"
          >
            <slot />
          </div>
          <div v-else class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
            <slot />
          </div>
          <footer
            v-if="$slots.footer"
            class="flex items-center justify-end gap-2 border-t border-line bg-panel px-4 py-3"
          >
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
