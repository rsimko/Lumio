<script setup lang="ts">
import { Check, EyeOff, Keyboard, Loader2, RotateCcw, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { formatCount, pluralize } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { MonoFont, ThemeSetting, UiFont } from '@/types';

const store = useLibraryStore();
const confirmingClear = ref(false);
const restoringId = ref<string | null>(null);

async function restoreHidden(id: string) {
  if (restoringId.value) return;
  restoringId.value = id;
  await store.restoreHiddenAssets([id]);
  restoringId.value = null;
}

async function restoreAllHidden() {
  if (restoringId.value || !store.hiddenAssets.length) return;
  restoringId.value = 'all';
  await store.restoreHiddenAssets(store.hiddenAssets.map((asset) => asset.id));
  restoringId.value = null;
}

const themes: { value: ThemeSetting; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
];

const uiFonts: { value: UiFont; label: string }[] = [
  { value: 'avenir', label: 'Avenir' },
  { value: 'system', label: 'System' },
];

const monoFonts: { value: MonoFont; label: string }[] = [
  { value: 'jetbrains', label: 'JetBrains Mono' },
  { value: 'system', label: 'System mono' },
];

function onThumbnailInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value)) store.setThumbnailSize(value);
}

function onFontSizeInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value)) store.setUiFontSize(value);
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
        <h3 class="panel-heading mb-2">Typography</h3>
        <div class="divide-y divide-line rounded-md border border-line bg-panel">
          <div class="px-3 py-2.5">
            <p class="text-xs font-medium text-ink">Interface font</p>
            <p class="mt-0.5 text-2xs text-subtle">
              Avenir is used when available, with the system UI font as a fallback.
            </p>
            <div class="mt-2 grid grid-cols-2 gap-1.5">
              <button
                v-for="font in uiFonts"
                :key="font.value"
                class="flex items-center justify-center gap-1.5 rounded-md border px-2 py-1.5 text-xs transition"
                :class="
                  store.settings.uiFont === font.value
                    ? 'border-accent bg-accent-soft font-medium text-ink'
                    : 'border-line bg-panel text-muted hover:bg-panel-2'
                "
                :aria-pressed="store.settings.uiFont === font.value"
                @click="store.setUiFont(font.value)"
              >
                <Check
                  v-if="store.settings.uiFont === font.value"
                  class="h-3 w-3 text-accent"
                />
                {{ font.label }}
              </button>
            </div>
          </div>

          <div class="px-3 py-2.5">
            <p class="text-xs font-medium text-ink">Monospace font</p>
            <p class="mt-0.5 text-2xs text-subtle">
              Used for file paths, cache locations and keyboard shortcuts.
            </p>
            <div class="mt-2 grid grid-cols-2 gap-1.5">
              <button
                v-for="font in monoFonts"
                :key="font.value"
                class="flex items-center justify-center gap-1.5 rounded-md border px-2 py-1.5 text-xs transition"
                :class="
                  store.settings.monoFont === font.value
                    ? 'border-accent bg-accent-soft font-medium text-ink'
                    : 'border-line bg-panel text-muted hover:bg-panel-2'
                "
                :aria-pressed="store.settings.monoFont === font.value"
                @click="store.setMonoFont(font.value)"
              >
                <Check
                  v-if="store.settings.monoFont === font.value"
                  class="h-3 w-3 text-accent"
                />
                {{ font.label }}
              </button>
            </div>
          </div>

          <div class="px-3 py-2.5">
            <div class="flex items-center justify-between gap-4">
              <div>
                <p class="text-xs font-medium text-ink">Text size</p>
                <p class="mt-0.5 text-2xs text-subtle">Scales labels and text without resizing panels.</p>
              </div>
              <span class="shrink-0 text-2xs tabular-nums text-muted">
                {{ store.settings.uiFontSize }} px
              </span>
            </div>
            <input
              type="range"
              min="11"
              max="17"
              step="1"
              class="mt-2 w-full cursor-pointer"
              style="accent-color: rgb(var(--c-accent))"
              :value="store.settings.uiFontSize"
              aria-label="Text size"
              @input="onFontSizeInput"
            />
            <p class="mt-2 rounded-md border border-line/70 bg-elevated px-2.5 py-2 text-xs text-muted">
              Preview —
              <span class="text-ink">The quick brown fox</span>
              <span class="text-subtle"> · </span>
              <span class="font-mono text-2xs">ISO 100 · 35 mm</span>
            </p>
          </div>
        </div>
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

          <div
            v-if="store.viewMode === 'filmstrip'"
            class="flex items-center justify-between gap-4 px-3 py-2.5"
          >
            <div>
              <p class="text-xs font-medium text-ink">Show filmstrip</p>
              <p class="mt-0.5 text-2xs text-subtle">Keep the navigation strip visible below the photo.</p>
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
        <div class="mb-2 flex items-center justify-between gap-2">
          <h3 class="panel-heading">Hidden photos</h3>
          <button
            v-if="store.hiddenAssets.length > 1"
            class="btn text-2xs"
            :disabled="restoringId !== null"
            @click="restoreAllHidden()"
          >
            <Loader2 v-if="restoringId === 'all'" class="h-3 w-3 animate-spin" />
            <RotateCcw v-else class="h-3 w-3" />
            Restore all
          </button>
        </div>
        <div class="rounded-md border border-line bg-panel">
          <div v-if="!store.hiddenAssets.length" class="flex items-start gap-2.5 px-3 py-3">
            <EyeOff class="mt-0.5 h-4 w-4 shrink-0 text-subtle" />
            <div>
              <p class="text-xs font-medium text-ink">No hidden photos</p>
              <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
                Photos removed with “Delete from library only” appear here. Their files stay on
                disk and can be restored at any time.
              </p>
            </div>
          </div>
          <div v-else class="divide-y divide-line">
            <div
              v-for="hidden in store.hiddenAssets"
              :key="hidden.id"
              class="flex items-center gap-3 px-3 py-2.5"
            >
              <div class="min-w-0 flex-1">
                <p class="truncate text-xs font-medium text-ink">{{ hidden.name }}</p>
                <p class="mt-0.5 break-all font-mono text-2xs text-subtle">
                  {{ hidden.primaryPath }}
                </p>
                <p class="mt-0.5 text-2xs text-subtle">
                  {{ formatCount(hidden.paths.length) }}
                  {{ pluralize(hidden.paths.length, 'file') }} hidden
                </p>
              </div>
              <button
                class="btn shrink-0 text-2xs"
                :disabled="restoringId !== null"
                @click="restoreHidden(hidden.id)"
              >
                <Loader2 v-if="restoringId === hidden.id" class="h-3 w-3 animate-spin" />
                <RotateCcw v-else class="h-3 w-3" />
                Restore
              </button>
            </div>
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
