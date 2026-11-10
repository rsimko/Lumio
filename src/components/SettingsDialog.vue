<script setup lang="ts">
import { Check, Keyboard, Loader2, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { useLibraryStore } from '@/stores/library';
import type { ThemeSetting } from '@/types';

const store = useLibraryStore();
const confirmingClear = ref(false);

const themes: { value: ThemeSetting; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
];

function onThumbnailInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value)) store.setThumbnailSize(value);
}

async function clearCache() {
  if (!confirmingClear.value) {
    confirmingClear.value = true;
    return;
  }
  await store.clearThumbnailCache();
  confirmingClear.value = false;
}

function openShortcuts() {
  store.closeSettings();
  store.openShortcuts();
}
</script>

<template>
  <AppDialog title="Settings & About" subtitle="Preferences are stored locally on this Mac." @close="store.closeSettings()">
    <div class="space-y-6">
      <section>
        <h3 class="panel-heading mb-2">Appearance</h3>
        <div class="grid grid-cols-3 gap-1.5">
          <button
            v-for="theme in themes"
            :key="theme.value"
            class="flex items-center justify-center gap-1.5 rounded-md border px-2 py-2 text-xs transition"
            :class="
              store.settings.theme === theme.value
                ? 'border-accent bg-accent-soft font-medium text-ink'
                : 'border-line bg-panel text-muted hover:bg-panel-2'
            "
            @click="store.setTheme(theme.value)"
          >
            <Check v-if="store.settings.theme === theme.value" class="h-3 w-3 text-accent" />
            {{ theme.label }}
          </button>
        </div>
        <p class="mt-1.5 text-2xs text-subtle">
          The System option follows the macOS appearance automatically.
        </p>
      </section>

      <section>
        <h3 class="panel-heading mb-2">Library</h3>
        <div class="divide-y divide-line rounded-md border border-line bg-panel">
          <div class="flex items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <p class="text-xs font-medium text-ink">Include subfolders</p>
              <p class="mt-0.5 text-2xs text-subtle">Scan nested folders when a folder is opened.</p>
            </div>
            <button
              class="switch"
              :class="store.settings.recursive ? 'switch-on' : ''"
              role="switch"
              :aria-checked="store.settings.recursive"
              @click="store.setRecursive(!store.settings.recursive)"
            >
              <span class="switch-knob" />
            </button>
          </div>

          <div class="px-3 py-2.5">
            <div class="flex items-center justify-between gap-4">
              <div>
                <p class="text-xs font-medium text-ink">Thumbnail size</p>
                <p class="mt-0.5 text-2xs text-subtle">Grid, masonry and justified layouts.</p>
              </div>
              <span class="shrink-0 text-2xs tabular-nums text-muted">
                {{ store.settings.thumbnailSize }} px
              </span>
            </div>
            <input
              type="range"
              min="120"
              max="480"
              step="4"
              class="mt-2 w-full cursor-pointer"
              style="accent-color: rgb(var(--c-accent))"
              :value="store.settings.thumbnailSize"
              @input="onThumbnailInput"
            />
          </div>

          <div class="flex items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <p class="text-xs font-medium text-ink">Show filmstrip</p>
              <p class="mt-0.5 text-2xs text-subtle">Keep the quick navigation strip visible.</p>
            </div>
            <button
              class="switch"
              :class="store.settings.filmstripVisible ? 'switch-on' : ''"
              role="switch"
              :aria-checked="store.settings.filmstripVisible"
              @click="store.setFilmstripVisible(!store.settings.filmstripVisible)"
            >
              <span class="switch-knob" />
            </button>
          </div>

          <div class="flex items-center justify-between gap-4 px-3 py-2.5">
            <div>
              <p class="text-xs font-medium text-ink">Show info panel</p>
              <p class="mt-0.5 text-2xs text-subtle">File details, associations and preview source.</p>
            </div>
            <button
              class="switch"
              :class="store.infoVisible ? 'switch-on' : ''"
              role="switch"
              :aria-checked="store.infoVisible"
              @click="store.toggleInfo()"
            >
              <span class="switch-knob" />
            </button>
          </div>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-2">Preview cache</h3>
        <div class="rounded-md border border-line bg-panel p-3">
          <p class="text-2xs uppercase tracking-wider text-subtle">Cache location</p>
          <p class="mt-1 break-all font-mono text-2xs text-muted selectable">{{ store.cacheDir }}</p>
          <p class="mt-2 text-2xs leading-relaxed text-subtle">
            Thumbnails are generated locally and can be rebuilt at any time. Clearing the cache
            never touches your photo files.
          </p>
          <div class="mt-2.5 flex items-center gap-2">
            <button
              class="btn text-danger hover:border-danger/40 hover:bg-danger/10"
              :disabled="store.clearingCache"
              @click="clearCache()"
            >
              <Loader2 v-if="store.clearingCache" class="h-3.5 w-3.5 animate-spin" />
              <Trash2 v-else class="h-3.5 w-3.5" />
              {{ confirmingClear ? 'Confirm clear cache' : 'Clear thumbnail cache' }}
            </button>
            <button v-if="confirmingClear" class="btn" @click="confirmingClear = false">
              Cancel
            </button>
          </div>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-2">About Lumio</h3>
        <div class="rounded-md border border-line bg-panel p-3">
          <div class="flex items-center justify-between">
            <p class="text-xs font-medium text-ink">Lumio {{ store.appVersion }}</p>
            <span class="badge bg-panel-2 text-muted ring-1 ring-line">GPL-3.0-or-later</span>
          </div>
          <p class="mt-1.5 text-2xs leading-relaxed text-subtle">
            Local-first RAW photo browser. No imports, no cloud, no modifications to originals.
            Metadata is associated from existing XMP sidecars; writing metadata is planned for a
            later milestone.
          </p>
          <p class="mt-2 text-2xs text-subtle">
            Supported: RAW (via rawler) · JPEG · PNG · TIFF · WebP. HEIC and AVIF are planned.
          </p>
          <button class="btn mt-2.5" @click="openShortcuts()">
            <Keyboard class="h-3.5 w-3.5" />
            Keyboard shortcuts
          </button>
        </div>
      </section>
    </div>

    <template #footer>
      <button class="btn" @click="store.closeSettings()">Close</button>
    </template>
  </AppDialog>
</template>
