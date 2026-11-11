<script setup lang="ts">
import { MoveDiagonal2, X } from 'lucide-vue-next';
import { computed, ref } from 'vue';

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

const MIN_WIDTH = 720;
const MIN_HEIGHT = 480;
const VIEWPORT_MARGIN = 24;
const SIZE_KEY = 'lumio.fullDialogSize';

interface DialogSize {
  width: number;
  height: number;
}

function readSavedSize(): DialogSize | null {
  try {
    const raw = window.localStorage.getItem(SIZE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<DialogSize>;
    if (typeof parsed.width !== 'number' || typeof parsed.height !== 'number') return null;
    return { width: parsed.width, height: parsed.height };
  } catch {
    return null;
  }
}

function saveSize(size: DialogSize | null): void {
  try {
    if (size) window.localStorage.setItem(SIZE_KEY, JSON.stringify(size));
    else window.localStorage.removeItem(SIZE_KEY);
  } catch {
    /* storage is best-effort */
  }
}

// Full dialogs remember a resized size for this machine.
const customSize = ref<DialogSize | null>(readSavedSize());
const resizing = ref(false);
const dialog = ref<HTMLElement | null>(null);

const dialogStyle = computed(() =>
  customSize.value
    ? {
        width: `${customSize.value.width}px`,
        height: `${customSize.value.height}px`,
        maxWidth: `calc(100vw - ${VIEWPORT_MARGIN}px)`,
        maxHeight: `calc(100vh - ${VIEWPORT_MARGIN}px)`,
      }
    : undefined,
);

function startResize(event: PointerEvent): void {
  const element = dialog.value;
  if (!props.full || !element) return;
  event.preventDefault();
  const rect = element.getBoundingClientRect();
  const startX = event.clientX;
  const startY = event.clientY;
  const startWidth = rect.width;
  const startHeight = rect.height;
  const maxWidth = window.innerWidth - VIEWPORT_MARGIN;
  const maxHeight = window.innerHeight - VIEWPORT_MARGIN;
  resizing.value = true;

  const move = (moveEvent: PointerEvent) => {
    customSize.value = {
      width: Math.min(maxWidth, Math.max(MIN_WIDTH, startWidth + moveEvent.clientX - startX)),
      height: Math.min(maxHeight, Math.max(MIN_HEIGHT, startHeight + moveEvent.clientY - startY)),
    };
  };
  const stop = () => {
    resizing.value = false;
    saveSize(customSize.value);
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', stop);
    window.removeEventListener('pointercancel', stop);
    window.removeEventListener('blur', stop);
  };
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', stop);
  window.addEventListener('pointercancel', stop);
  window.addEventListener('blur', stop);
}
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog" appear>
      <div
        class="fixed inset-0 z-40 flex items-center justify-center bg-black/45 backdrop-blur-[2px]"
        :class="props.full ? 'p-3' : 'p-6'"
        @pointerdown.self="$emit('close')"
      >
        <div
          ref="dialog"
          class="flex flex-col overflow-hidden rounded-xl border border-line bg-elevated shadow-float"
          :class="[
            props.full
              ? 'relative h-[92vh] w-[min(1760px,calc(100vw-24px))]'
              : ['w-full max-h-[86vh]', props.wide ? 'max-w-2xl' : 'max-w-lg'],
            resizing ? 'select-none' : '',
          ]"
          :style="dialogStyle"
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
          <div v-if="flush" class="flex min-h-0 flex-1 flex-col overflow-hidden">
            <slot />
          </div>
          <div v-else class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-4 py-4">
            <slot />
          </div>
          <footer
            v-if="$slots.footer"
            class="flex items-center justify-end gap-2 border-t border-line bg-panel px-4 py-3"
          >
            <slot name="footer" />
          </footer>

          <button
            v-if="props.full"
            type="button"
            class="absolute bottom-0 right-0 z-20 flex h-5 w-5 cursor-nwse-resize touch-none items-end justify-end p-0.5 text-subtle/70 transition hover:text-muted"
            title="Drag to resize · double-click to reset"
            aria-label="Resize dialog"
            @pointerdown.stop="startResize"
            @dblclick.stop="
              customSize = null;
              saveSize(null);
            "
          >
            <MoveDiagonal2 class="h-3 w-3" />
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
