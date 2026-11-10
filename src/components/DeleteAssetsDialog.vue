<script setup lang="ts">
import { AlertTriangle, Check, Images, Loader2, Trash2, FolderMinus } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import { collectAssetFilePaths, sharedAssociationPaths } from '@/lib/assets';
import { formatCount, pluralize } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { AssetRemovalMode } from '@/types';

const store = useLibraryStore();

const mode = ref<AssetRemovalMode>('libraryOnly');
const acknowledged = ref(false);

const targets = computed(() => store.deleteAssetsTargets);
const photoCount = computed(() => targets.value.length);
const fileCount = computed(() => collectAssetFilePaths(targets.value).length);
const sharedFiles = computed(() => sharedAssociationPaths(store.assets, targets.value));
const isDestructive = computed(() => mode.value === 'moveToTrash');
const canConfirm = computed(() => {
  if (!targets.value.length || store.removingAssets) return false;
  if (!isDestructive.value) return true;
  return acknowledged.value;
});
const title = computed(() =>
  photoCount.value === 1 ? targets.value[0]?.fileName ?? 'photo' : `${photoCount.value} photos`,
);
const confirmLabel = computed(() =>
  isDestructive.value
    ? `Move ${photoCount.value === 1 ? 'photo' : 'photos'} to Trash`
    : `Remove ${photoCount.value === 1 ? 'photo' : 'photos'} from library`,
);

async function confirm() {
  if (!canConfirm.value) return;
  await store.removeSelectedAssets(mode.value);
}
</script>

<template>
  <AppDialog
    title="Delete photos"
    subtitle="Choose whether Lumio forgets these photos or moves their files to the Trash."
    @close="store.closeDeleteAssets()"
  >
    <div class="space-y-4">
      <div class="flex items-start gap-3 rounded-lg border border-line bg-panel px-3 py-3">
        <div class="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-panel-2 text-muted">
          <Images class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <p class="truncate text-xs font-semibold text-ink">{{ title }}</p>
          <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
            {{ formatCount(photoCount) }} {{ pluralize(photoCount, 'photo') }} ·
            {{ formatCount(fileCount) }} {{ pluralize(fileCount, 'file') }} including companion
            JPEGs, XMP sidecars and other associated files.
          </p>
          <p
            v-if="photoCount === 1 && targets[0]"
            class="mt-1 break-all font-mono text-2xs text-subtle selectable"
          >
            {{ targets[0].primaryPath }}
          </p>
          <p v-else class="mt-1 line-clamp-2 text-2xs text-subtle">
            {{ targets.map((asset) => asset.fileName).join(', ') }}
          </p>
        </div>
      </div>

      <div class="space-y-2">
        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            !isDestructive
              ? 'border-accent bg-accent-soft/60'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="!isDestructive"
          @click="mode = 'libraryOnly'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="!isDestructive ? 'bg-accent/15 text-accent' : 'bg-panel-2 text-muted'"
          >
            <FolderMinus class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Delete from library only
              <span class="badge bg-accent/10 text-accent">Recommended</span>
              <Check v-if="!isDestructive" class="ml-auto h-3.5 w-3.5 shrink-0 text-accent" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Hides these photos and their associated files from Lumio. Nothing on disk changes.
              Restore them later in Settings → Hidden photos.
            </span>
          </span>
        </button>

        <button
          type="button"
          class="flex w-full items-start gap-3 rounded-lg border px-3 py-3 text-left transition"
          :class="
            isDestructive
              ? 'border-danger/50 bg-danger/5'
              : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2'
          "
          :aria-pressed="isDestructive"
          @click="mode = 'moveToTrash'"
        >
          <span
            class="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md"
            :class="isDestructive ? 'bg-danger/15 text-danger' : 'bg-panel-2 text-muted'"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5 text-xs font-semibold text-ink">
              Delete from disk
              <Check v-if="isDestructive" class="ml-auto h-3.5 w-3.5 shrink-0 text-danger" />
            </span>
            <span class="mt-1 block text-2xs leading-relaxed text-muted">
              Moves every associated file to the system Trash. You can restore them from the Trash
              until you empty it.
            </span>
          </span>
        </button>
      </div>

      <div
        v-if="sharedFiles.length"
        class="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2.5"
      >
        <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-warning" />
        <p class="text-2xs leading-relaxed text-ink">
          {{ formatCount(sharedFiles.length) }}
          {{ pluralize(sharedFiles.length, 'associated file is', 'associated files are') }} also
          linked to other visible photos (for example a shared XMP sidecar or JPEG). Those links
          will disappear too.
        </p>
      </div>

      <div
        v-if="isDestructive"
        class="space-y-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-3"
      >
        <div class="flex items-start gap-2">
          <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-danger" />
          <p class="text-xs leading-relaxed text-ink">
            <span class="font-semibold text-danger">
              This moves {{ formatCount(fileCount) }} {{ pluralize(fileCount, 'file') }} to the
              Trash.
            </span>
            The primary photos, companion JPEGs, XMP sidecars and any extra associated files leave
            their current location together.
          </p>
        </div>

        <label
          class="flex cursor-pointer items-start gap-2 rounded-md border border-danger/20 bg-elevated/70 px-2.5 py-2 text-xs leading-relaxed text-ink"
        >
          <input
            v-model="acknowledged"
            type="checkbox"
            class="mt-0.5 h-3.5 w-3.5 shrink-0 accent-danger"
          />
          <span>I understand these files will be moved to the Trash, not permanently erased.</span>
        </label>
      </div>

      <p
        v-if="store.removeAssetsError"
        class="rounded-md border border-danger/30 bg-danger/5 px-3 py-2 text-xs leading-relaxed text-danger"
      >
        {{ store.removeAssetsError }}
      </p>
    </div>

    <template #footer>
      <button class="btn" :disabled="store.removingAssets" @click="store.closeDeleteAssets()">
        Cancel
      </button>
      <button
        :class="isDestructive ? 'btn-danger' : 'btn-primary'"
        :disabled="!canConfirm"
        @click="confirm"
      >
        <Loader2 v-if="store.removingAssets" class="h-3.5 w-3.5 animate-spin" />
        <Trash2 v-else-if="isDestructive" class="h-3.5 w-3.5" />
        <FolderMinus v-else class="h-3.5 w-3.5" />
        {{ confirmLabel }}
      </button>
    </template>
  </AppDialog>
</template>
