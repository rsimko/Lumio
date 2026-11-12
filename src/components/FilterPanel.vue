<script setup lang="ts">
import { CircleSlash, Flag, FolderOpen, Heart, RotateCcw, Star, X } from 'lucide-vue-next';
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

const canFilter = computed(() => Boolean(store.selectedFolder));
const active = computed(() => store.hasActiveFilters);
const resultLabel = computed(
  () =>
    `${formatCount(store.filteredCount)} of ${formatCount(store.totalCount)} ${
      store.totalCount === 1 ? 'photo' : 'photos'
    }`,
);

function chipClass(selected: boolean): string {
  return selected
    ? 'border-accent/40 bg-accent-soft text-accent'
    : 'border-line bg-main text-muted hover:border-line-strong hover:bg-panel-2 hover:text-ink';
}

function ratingActive(value: RatingFilterValue): boolean {
  return store.filters.ratings.includes(value);
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

function favoriteActive(value: FavoriteFilterValue): boolean {
  return store.filters.favorites.includes(value);
}

function ratingTitle(value: RatingFilterValue): string {
  if (value === 0) return 'Show unrated photos';
  return `Show photos rated ${value} ${value === 1 ? 'star' : 'stars'}`;
}

function cullTitle(value: CullFilterValue): string {
  if (value === 'pick') return 'Show picked photos';
  if (value === 'reject') return 'Show rejected photos';
  return 'Show photos without a culling flag';
}
</script>

<template>
  <div class="space-y-4 px-2 py-2">
    <div class="flex items-start gap-2">
      <div class="min-w-0 flex-1">
        <p class="text-xs font-medium text-ink">Filter the current folder</p>
        <p v-if="canFilter" class="mt-0.5 text-2xs text-subtle">
          {{ resultLabel }}<span v-if="active"> · {{ store.activeFilterCount }} active</span>
        </p>
        <p v-else class="mt-0.5 text-2xs text-subtle">Choose a folder first.</p>
      </div>
      <button
        class="btn-icon-sm"
        :disabled="!active"
        title="Clear all filters"
        aria-label="Clear all filters"
        @click="store.clearFilters()"
      >
        <RotateCcw class="h-3.5 w-3.5" />
      </button>
    </div>

    <div
      v-if="!canFilter"
      class="rounded-md border border-line bg-main/60 px-3 py-4 text-center"
    >
      <FolderOpen class="mx-auto mb-2 h-5 w-5 text-subtle" />
      <p class="text-2xs leading-relaxed text-subtle">
        Filters apply to the folder you are browsing.
      </p>
      <button class="btn mt-3 w-full justify-center" @click="store.setLeftPanelTab('library')">
        Open folders
      </button>
    </div>

    <template v-else>
      <section>
        <h3 class="panel-heading mb-1.5">Rating</h3>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="value in RATING_FILTER_VALUES"
            :key="value"
            type="button"
            class="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-2xs font-medium transition"
            :class="chipClass(ratingActive(value))"
            :title="ratingTitle(value)"
            :aria-pressed="ratingActive(value)"
            @click="store.toggleRatingFilter(value)"
          >
            <Star
              v-if="value > 0"
              class="h-3 w-3"
              :class="ratingActive(value) ? 'text-amber-400' : 'text-subtle/70'"
              :fill="ratingActive(value) ? 'currentColor' : 'none'"
            />
            <Star
              v-else
              class="h-3 w-3 text-subtle/70"
              fill="none"
            />
            {{ value === 0 ? 'Unrated' : value }}
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Color label</h3>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="label in LABEL_FILTER_VALUES"
            :key="label"
            type="button"
            class="inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-2xs font-medium transition"
            :class="chipClass(labelActive(label))"
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
            {{ labelFilterName(label) }}
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Culling</h3>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="value in CULL_FILTER_VALUES"
            :key="value"
            type="button"
            class="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-2xs font-medium capitalize transition"
            :class="chipClass(cullActive(value))"
            :title="cullTitle(value)"
            :aria-pressed="cullActive(value)"
            @click="store.toggleCullFilter(value)"
          >
            <Flag
              v-if="value === 'pick'"
              class="h-3 w-3"
              :fill="cullActive(value) ? 'currentColor' : 'none'"
            />
            <X v-else-if="value === 'reject'" class="h-3 w-3" />
            <CircleSlash v-else class="h-3 w-3" />
            {{ value === 'unflagged' ? 'Unflagged' : value }}
          </button>
        </div>
      </section>

      <section>
        <h3 class="panel-heading mb-1.5">Favorites</h3>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="value in FAVORITE_FILTER_VALUES"
            :key="value"
            type="button"
            class="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-2xs font-medium transition"
            :class="chipClass(favoriteActive(value))"
            :aria-pressed="favoriteActive(value)"
            @click="store.toggleFavoriteFilter(value)"
          >
            <Heart
              class="h-3 w-3"
              :class="value === 'favorite' ? 'text-rose-500' : 'text-subtle/70'"
              :fill="value === 'favorite' && favoriteActive(value) ? 'currentColor' : 'none'"
            />
            {{ value === 'favorite' ? 'Favorites' : 'Not favorite' }}
          </button>
        </div>
      </section>

      <button
        class="btn w-full justify-center"
        :disabled="!active"
        @click="store.clearFilters()"
      >
        <RotateCcw class="h-3.5 w-3.5" />
        Clear all filters
      </button>

      <p class="text-2xs leading-relaxed text-subtle">
        Values in one group match any selection. Different groups must all match.
        <template v-if="store.activeFilterCount">
          {{ store.activeFilterCount }}
          {{ store.activeFilterCount === 1 ? 'filter is' : 'filters are' }} active.
        </template>
      </p>
    </template>
  </div>
</template>
