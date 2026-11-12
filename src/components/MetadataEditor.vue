<script setup lang="ts">
import {
  AlertTriangle,
  Flag,
  Heart,
  Loader2,
  Star,
  X,
} from 'lucide-vue-next';
import { computed } from 'vue';

import InfoSection from '@/components/InfoSection.vue';
import {
  COLOR_LABELS,
  COLOR_LABEL_NAMES,
  COLOR_LABEL_SWATCHES,
  commonMetadataValue,
  metadataSourceLabel,
} from '@/lib/metadata';
import { useLibraryStore } from '@/stores/library';
import type { ColorLabel, CullFlag } from '@/types';

const store = useLibraryStore();

const selection = computed(() => store.selectedAssets);
const count = computed(() => selection.value.length);
const single = computed(() => (count.value === 1 ? selection.value[0] : null));

const rating = computed(() => commonMetadataValue(selection.value, (value) => value.rating, 0));
const label = computed(() => commonMetadataValue(selection.value, (value) => value.label, null));
const cull = computed(() => commonMetadataValue(selection.value, (value) => value.cull, null));
const favorite = computed(() =>
  commonMetadataValue(selection.value, (value) => value.favorite, false),
);

const issues = computed(() => single.value?.metadata.issues ?? []);
const keywords = computed(() => single.value?.metadata.keywords ?? []);
const hierarchicalKeywords = computed(() => single.value?.metadata.hierarchicalKeywords ?? []);
const source = computed(() =>
  single.value ? metadataSourceLabel(single.value.metadata.source) : 'Multiple photos',
);
const busy = computed(() => store.metadataBusy);
const anyMixed = computed(
  () => rating.value.mixed || label.value.mixed || cull.value.mixed || favorite.value.mixed,
);

function starFilled(value: number): boolean {
  return !rating.value.mixed && rating.value.value >= value;
}

function labelActive(value: ColorLabel): boolean {
  return !label.value.mixed && label.value.value === value;
}

function cullActive(value: CullFlag): boolean {
  return !cull.value.mixed && cull.value.value === value;
}

function setRating(value: number): void {
  if (!busy.value && count.value) store.setRating(value);
}

function clearRating(): void {
  if (!busy.value && count.value) store.setRating(0);
}

function setLabel(value: ColorLabel): void {
  if (!busy.value && count.value) store.setColorLabel(value);
}

function clearLabel(): void {
  if (!busy.value && count.value) store.setColorLabel(null);
}

function setCull(value: CullFlag): void {
  if (!busy.value && count.value) store.setCull(value);
}

function unflag(): void {
  if (!busy.value && count.value) store.unflagSelection();
}

function toggleFavorite(): void {
  if (!busy.value && count.value) store.toggleFavorite();
}
</script>

<template>
  <InfoSection title="Metadata">
    <template #actions>
      <span class="flex items-center gap-1.5 text-2xs text-subtle">
        <Loader2 v-if="busy" class="h-3 w-3 animate-spin text-accent" />
        <span v-if="count > 1">
          {{ count }} photos<span v-if="anyMixed"> · Mixed</span>
        </span>
      </span>
    </template>

    <div class="space-y-2.5">
      <div class="flex min-h-7 items-center gap-2">
        <span class="w-12 shrink-0 text-2xs text-subtle">Rating</span>
        <div class="flex min-w-0 flex-1 items-center justify-between gap-2">
          <div class="flex items-center gap-0.5" role="group" aria-label="Star rating">
            <button
              v-for="value in [1, 2, 3, 4, 5]"
              :key="value"
              class="grid h-6 w-6 place-items-center rounded-md transition hover:bg-panel-2"
              :class="starFilled(value) ? 'text-amber-400' : 'text-subtle/60'"
              :title="
                !rating.mixed && rating.value === value
                  ? `Clear ${value}-star rating`
                  : `Rate ${value} ${value === 1 ? 'star' : 'stars'}`
              "
              :aria-label="`Rate ${value} ${value === 1 ? 'star' : 'stars'}`"
              :aria-pressed="starFilled(value)"
              @click="setRating(value)"
            >
              <Star
                class="h-3.5 w-3.5"
                :fill="starFilled(value) ? 'currentColor' : 'none'"
                :stroke-width="starFilled(value) ? 1.5 : 1.75"
              />
            </button>
          </div>
          <button
            class="rounded px-1 py-0.5 text-2xs font-medium text-subtle transition hover:bg-panel-2 hover:text-ink disabled:pointer-events-none disabled:opacity-40"
            :title="
              rating.mixed
                ? 'Clear rating for all selected photos'
                : rating.value
                  ? `Clear ${rating.value}-star rating`
                  : 'No rating'
            "
            :disabled="!rating.mixed && rating.value === 0"
            aria-label="Clear rating"
            @click="clearRating"
          >
            Clear
          </button>
        </div>
      </div>

      <div class="flex min-h-7 items-center gap-2">
        <span class="w-12 shrink-0 text-2xs text-subtle">Label</span>
        <div class="flex min-w-0 flex-1 items-center justify-between gap-2">
          <div class="flex items-center gap-0.5" role="group" aria-label="Color label">
            <button
              v-for="color in COLOR_LABELS"
              :key="color"
              class="group grid h-6 w-6 place-items-center rounded-md transition hover:bg-panel-2"
              :title="
                labelActive(color)
                  ? `Clear ${COLOR_LABEL_NAMES[color].toLowerCase()} label`
                  : `Set ${COLOR_LABEL_NAMES[color].toLowerCase()} label`
              "
              :aria-label="
                labelActive(color)
                  ? `Clear ${COLOR_LABEL_NAMES[color]} label`
                  : `Set ${COLOR_LABEL_NAMES[color]} label`
              "
              :aria-pressed="labelActive(color)"
              @click="setLabel(color)"
            >
              <span
                class="block h-4 w-4 rounded-full ring-2 ring-offset-1 ring-offset-panel transition group-hover:scale-110"
                :class="[
                  COLOR_LABEL_SWATCHES[color],
                  labelActive(color)
                    ? ['ring-ink/70', 'scale-110']
                    : ['ring-transparent', 'opacity-75 group-hover:opacity-100'],
                ]"
              />
            </button>
          </div>
          <button
            class="rounded px-1 py-0.5 text-2xs font-medium text-subtle transition hover:bg-panel-2 hover:text-ink disabled:pointer-events-none disabled:opacity-40"
            :title="
              label.mixed
                ? 'Clear labels for all selected photos'
                : label.value
                  ? `Clear ${COLOR_LABEL_NAMES[label.value].toLowerCase()} label`
                  : 'No label'
            "
            :disabled="!label.mixed && !label.value"
            aria-label="Clear color label"
            @click="clearLabel"
          >
            Clear
          </button>
        </div>
      </div>

      <div class="flex min-h-7 items-center gap-2">
        <span class="w-12 shrink-0 text-2xs text-subtle">Culling</span>
        <div class="flex min-w-0 flex-1 items-center justify-between gap-2">
          <div class="flex items-center gap-0.5" role="group" aria-label="Culling flag">
            <button
              class="grid h-6 w-6 place-items-center rounded-md transition hover:bg-panel-2"
              :class="cullActive('pick') ? 'text-success' : 'text-subtle/60'"
              :title="cullActive('pick') ? 'Clear pick flag' : 'Mark as picked'"
              :aria-label="cullActive('pick') ? 'Clear pick flag' : 'Mark as picked'"
              :aria-pressed="cullActive('pick')"
              @click="setCull('pick')"
            >
              <Flag
                class="h-3.5 w-3.5"
                :fill="cullActive('pick') ? 'currentColor' : 'none'"
              />
            </button>
            <button
              class="grid h-6 w-6 place-items-center rounded-md transition hover:bg-panel-2"
              :class="cullActive('reject') ? 'text-danger' : 'text-subtle/60'"
              :title="cullActive('reject') ? 'Clear reject flag' : 'Mark as rejected'"
              :aria-label="cullActive('reject') ? 'Clear reject flag' : 'Mark as rejected'"
              :aria-pressed="cullActive('reject')"
              @click="setCull('reject')"
            >
              <X class="h-3.5 w-3.5" />
            </button>
          </div>
          <button
            class="rounded px-1 py-0.5 text-2xs font-medium text-subtle transition hover:bg-panel-2 hover:text-ink disabled:pointer-events-none disabled:opacity-40"
            :title="
              cull.mixed
                ? 'Clear culling flags for all selected photos'
                : cull.value
                  ? `Clear ${cull.value === 'pick' ? 'pick' : 'reject'} flag`
                  : 'No culling flag'
            "
            :disabled="!cull.mixed && !cull.value"
            aria-label="Clear culling flag"
            @click="unflag"
          >
            Clear
          </button>
        </div>
      </div>

      <div class="flex min-h-7 items-center gap-2">
        <span class="w-12 shrink-0 text-2xs text-subtle">Favorite</span>
        <div class="flex min-w-0 flex-1 items-center justify-between gap-2">
          <div class="flex items-center gap-0.5" role="group" aria-label="Favorite">
            <button
              class="grid h-6 w-6 place-items-center rounded-md transition hover:bg-panel-2"
              :class="
                !favorite.mixed && favorite.value ? 'text-rose-500' : 'text-subtle/60'
              "
              :title="
                !favorite.mixed && favorite.value
                  ? `Remove favorite (${store.shortcutLabel('metadata.favorite')})`
                  : `Mark as favorite (${store.shortcutLabel('metadata.favorite')})`
              "
              :aria-label="
                !favorite.mixed && favorite.value ? 'Remove favorite' : 'Mark as favorite'
              "
              :aria-pressed="!favorite.mixed && favorite.value"
              @click="toggleFavorite"
            >
              <Heart
                class="h-3.5 w-3.5"
                :fill="!favorite.mixed && favorite.value ? 'currentColor' : 'none'"
              />
            </button>
          </div>
        </div>
      </div>
    </div>

    <p v-if="count > 1" class="mt-2 text-2xs leading-relaxed text-subtle">
      Changes apply to all {{ count }} selected photos.
    </p>

    <div
      v-if="single"
      class="mt-2 flex items-center gap-1.5 text-2xs text-subtle"
      :title="single.metadata.source === 'none' ? 'Metadata will be created on first edit.' : ''"
    >
      <span
        class="h-1.5 w-1.5 rounded-full"
        :class="
          single.metadata.source === 'sidecar'
            ? 'bg-accent'
            : single.metadata.source === 'embedded'
              ? 'bg-success'
              : 'bg-subtle/40'
        "
      />
      {{ source }}
    </div>

    <div
      v-for="issue in issues"
      :key="issue.code + issue.message"
      class="mt-2 flex items-start gap-1.5 rounded-md border border-warning/30 bg-warning/10 px-2 py-1.5 text-2xs leading-relaxed text-warning"
    >
      <AlertTriangle class="mt-px h-3 w-3 shrink-0" />
      {{ issue.message }}
    </div>

    <div v-if="store.metadataError" class="mt-2 text-2xs leading-relaxed text-danger">
      {{ store.metadataError }}
    </div>

    <div
      v-if="single && (keywords.length || hierarchicalKeywords.length)"
      class="mt-2.5 border-t border-line/60 pt-2"
    >
      <div class="mb-1 flex items-center justify-between">
        <span class="text-2xs text-subtle">Keywords</span>
        <span class="text-3xs uppercase tracking-wide text-subtle/70">Read-only</span>
      </div>
      <div class="flex flex-wrap gap-1">
        <span
          v-for="keyword in keywords"
          :key="`flat-${keyword}`"
          class="rounded bg-panel-2 px-1.5 py-0.5 text-2xs text-muted"
        >
          {{ keyword }}
        </span>
        <span
          v-for="keyword in hierarchicalKeywords"
          :key="`hier-${keyword}`"
          class="rounded bg-accent-soft px-1.5 py-0.5 text-2xs text-accent"
          :title="`Hierarchical keyword: ${keyword}`"
        >
          {{ keyword.replaceAll('|', ' › ') }}
        </span>
      </div>
    </div>
  </InfoSection>
</template>
