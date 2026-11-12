<script setup lang="ts">
import {
  Check,
  CircleSlash,
  Flag,
  FolderOpen,
  Heart,
  Images,
  Loader2,
  RotateCcw,
  X,
} from 'lucide-vue-next';
import { computed } from 'vue';

import {
  CULL_FILTER_VALUES,
  FAVORITE_FILTER_VALUES,
  LABEL_FILTER_VALUES,
  RATING_FILTER_VALUES,
  labelFilterName,
  type CullFilterValue,
  type FavoriteFilterValue,
  type LabelFilterValue,
  type RatingFilterValue,
} from '@/lib/filters';
import { formatCount } from '@/lib/format';
import { COLOR_LABEL_SWATCHES } from '@/lib/metadata';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

const folderAvailable = computed(() => Boolean(store.selectedFolder));
const libraryAvailable = computed(() => store.roots.length > 0);
const canFilter = computed(() =>
  store.isLibraryScope ? libraryAvailable.value : folderAvailable.value,
);
const active = computed(() => store.hasActiveFilters);
const scanning = computed(() => store.scanState === 'loading');
const resultLabel = computed(() => {
  if (scanning.value && store.totalCount === 0) {
    return store.isLibraryScope ? 'Scanning the library…' : 'Scanning the folder…';
  }
  return `${formatCount(store.filteredCount)} of ${formatCount(store.totalCount)} ${
    store.totalCount === 1 ? 'photo' : 'photos'
  }`;
});

function rowClass(selected: boolean): string {
  return selected
    ? 'rounded-full border-accent/40 bg-accent-soft text-accent'
    : 'rounded-md border-transparent bg-transparent text-muted hover:bg-panel-2 hover:text-ink';
}

function scopeUnavailable(scope: 'folder' | 'library'): boolean {
  return scope === 'library' ? !libraryAvailable.value : !folderAvailable.value;
}

function ratingActive(value: RatingFilterValue): boolean {
  return store.filters.ratings.includes(value);
}

function ratingLabel(value: RatingFilterValue): string {
  if (value === 0) return 'Unrated';
  return '★'.repeat(value);
}

function ratingTitle(value: RatingFilterValue): string {
  if (value === 0) return 'Show unrated photos';
  return `Show photos rated exactly ${value} ${value === 1 ? 'star' : 'stars'}`;
}

function labelActive(value: LabelFilterValue): boolean {
  return store.filters.labels.includes(value);
}

function labelSwatch(value: LabelFilterValue): string {
  return value === 'unlabeled' ? '' : COLOR_LABEL_SWATCHES[value];
}

function cullActive(value: CullFilterValue): boolean {
  return store.filters.culls.includes(value);
}

function cullTitle(value: CullFilterValue): string {
  if (value === 'pick') return 'Show picked photos';
  if (value === 'reject') return 'Show rejected photos';
  return 'Show photos without a culling flag';
}

function favoriteActive(value: FavoriteFilterValue): boolean {
  return store.filters.favorites.includes(value);
}
</script>

<template>
  <div class="space-y-4 px-2 py-2">
    <div class="flex items-start gap-2">
      <div class="min-w-0 flex-1">
        <p class="text-xs font-medium text-ink">Filter scope</p>
        <p v-if="canFilter" class="mt-0.5 flex items-center gap-1 text-2xs text-subtle">
          <Loader2 v-if="scanning" class="h-3 w-3 shrink-0 animate-spin" />
          {{ resultLabel }}<span v-if="active"> · {{ store.activeFilterCount }} active</span>
        </p>
        <p v-else class="mt-0.5 text-2xs text-subtle">Choose a folder or add one first.</p>
      </div>
      <button
        class="inline-flex h-6 shrink-0 items-center gap-1 rounded-full px-2 text-2xs font-medium text-muted transition hover:bg-panel-2 hover:text-ink disabled:pointer-events-none disabled:opacity-40"
        :disabled="!active"
        title="Clear all filters"
        @click="store.clearFilters()"
      >
        <RotateCcw class="h-3 w-3" />
        Clear
      </button>
    </div>

    <div class="space-y-1">
      <button
        type="button"
        class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left text-xs font-medium transition"
        :class="[
          rowClass(!store.isLibraryScope),
          scopeUnavailable('folder') ? 'cursor-not-allowed opacity-40' : '',
        ]"
        :disabled="scopeUnavailable('folder')"
        :aria-pressed="!store.isLibraryScope"
        @click="store.setFilterScope('folder')"
      >
        <FolderOpen class="h-3.5 w-3.5 shrink-0" />
        <span class="min-w-0 flex-1">
          <span class="block truncate">Current folder</span>
          <span class="mt-0.5 block truncate text-2xs font-normal text-subtle">
            {{ store.selectedFolder ? store.folderName : 'No folder selected' }}
          </span>
        </span>
        <Check
          class="h-3.5 w-3.5 shrink-0"
          :class="!store.isLibraryScope ? 'opacity-100' : 'opacity-0'"
        />
      </button>

      <button
        type="button"
        class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left text-xs font-medium transition"
        :class="[
          rowClass(store.isLibraryScope),
          scopeUnavailable('library') ? 'cursor-not-allowed opacity-40' : '',
        ]"
        :disabled="scopeUnavailable('library')"
        :aria-pressed="store.isLibraryScope"
        @click="store.setFilterScope('library')"
      >
        <Images class="h-3.5 w-3.5 shrink-0" />
        <span class="min-w-0 flex-1">
          <span class="block truncate">Entire library</span>
          <span class="mt-0.5 block truncate text-2xs font-normal text-subtle">
            {{ store.roots.length }}
            {{ store.roots.length === 1 ? 'folder' : 'folders' }}
          </span>
        </span>
        <Check
          class="h-3.5 w-3.5 shrink-0"
          :class="store.isLibraryScope ? 'opacity-100' : 'opacity-0'"
        />
      </button>
    </div>

    <div
      v-if="!canFilter"
      class="rounded-md border border-line bg-main/60 px-3 py-4 text-center"
    >
      <FolderOpen class="mx-auto mb-2 h-5 w-5 text-subtle" />
      <p class="text-2xs leading-relaxed text-subtle">
        Filters apply to the folder you are browsing or across the whole library.
      </p>
      <button class="btn mt-3 w-full justify-center" @click="store.setLeftPanelTab('library')">
        Open folders
      </button>
    </div>

    <template v-else>
      <section>
        <h3 class="panel-heading mb-1.5">Rating</h3>
        <div class="space-y-1">
          <button
            v-for="value in RATING_FILTER_VALUES"
            :key="value"
            type="button"
            class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left transition"
            :class="rowClass(ratingActive(value))"
            :title="ratingTitle(value)"
            :aria-label="ratingTitle(value)"
            :aria-pressed="ratingActive(value)"
            @click="store.toggleRatingFilter(value)"
          >
            <span
              class="min-w-0 flex-1 text-xs font-medium tracking-[0.12em]"
              :class="ratingActive(value) && value > 0 ? 'text-amber-400' : ''"
            >
              {{ ratingLabel(value) }}
            </span>
            <Check
              class="h-3.5 w-3.5 shrink-0"
              :class="ratingActive(value) ? 'opacity-100' : 'opacity-0'"
            />
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Color label</h3>
        <div class="space-y-1">
          <button
            v-for="label in LABEL_FILTER_VALUES"
            :key="label"
            type="button"
            class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left text-xs font-medium transition"
            :class="rowClass(labelActive(label))"
            :aria-pressed="labelActive(label)"
            @click="store.toggleLabelFilter(label)"
          >
            <span
              v-if="label !== 'unlabeled'"
              class="h-2.5 w-2.5 shrink-0 rounded-full"
              :class="labelSwatch(label)"
            />
            <span
              v-else
              class="h-2.5 w-2.5 shrink-0 rounded-full border border-subtle/70"
            />
            <span class="min-w-0 flex-1 truncate">{{ labelFilterName(label) }}</span>
            <Check
              class="h-3.5 w-3.5 shrink-0"
              :class="labelActive(label) ? 'opacity-100' : 'opacity-0'"
            />
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Culling</h3>
        <div class="space-y-1">
          <button
            v-for="value in CULL_FILTER_VALUES"
            :key="value"
            type="button"
            class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left text-xs font-medium capitalize transition"
            :class="rowClass(cullActive(value))"
            :title="cullTitle(value)"
            :aria-pressed="cullActive(value)"
            @click="store.toggleCullFilter(value)"
          >
            <Flag
              v-if="value === 'pick'"
              class="h-3.5 w-3.5 shrink-0"
              :fill="cullActive(value) ? 'currentColor' : 'none'"
            />
            <X v-else-if="value === 'reject'" class="h-3.5 w-3.5 shrink-0" />
            <CircleSlash v-else class="h-3.5 w-3.5 shrink-0" />
            <span class="min-w-0 flex-1 truncate">
              {{ value === 'unflagged' ? 'Unflagged' : value }}
            </span>
            <Check
              class="h-3.5 w-3.5 shrink-0"
              :class="cullActive(value) ? 'opacity-100' : 'opacity-0'"
            />
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Favorites</h3>
        <div class="space-y-1">
          <button
            v-for="value in FAVORITE_FILTER_VALUES"
            :key="value"
            type="button"
            class="flex w-full items-center gap-2 border px-2.5 py-1.5 text-left text-xs font-medium transition"
            :class="rowClass(favoriteActive(value))"
            :aria-pressed="favoriteActive(value)"
            @click="store.toggleFavoriteFilter(value)"
          >
            <Heart
              class="h-3.5 w-3.5 shrink-0"
              :class="value === 'favorite' ? 'text-rose-500' : 'text-subtle/70'"
              :fill="value === 'favorite' && favoriteActive(value) ? 'currentColor' : 'none'"
            />
            <span class="min-w-0 flex-1 truncate">
              {{ value === 'favorite' ? 'Favorites' : 'Not favorite' }}
            </span>
            <Check
              class="h-3.5 w-3.5 shrink-0"
              :class="favoriteActive(value) ? 'opacity-100' : 'opacity-0'"
            />
          </button>
        </div>
      </section>
    </template>
  </div>
</template>
