<script setup lang="ts">
import {
  AlertTriangle,
  CircleSlash,
  Flag,
  Heart,
  Loader2,
  Star,
  X,
} from 'lucide-vue-next';
import { computed } from 'vue';

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

function setLabel(value: ColorLabel): void {
  if (!busy.value && count.value) store.setColorLabel(value);
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
  <section class="border-t border-line px-3 py-3" aria-label="Photo metadata">
    <div class="mb-2 flex min-h-4 items-center justify-between gap-2">
      <h4 class="panel-heading">Metadata</h4>
      <span class="flex items-center gap-1.5 text-2xs text-subtle">
        <Loader2 v-if="busy" class="h-3 w-3 animate-spin text-accent" />
        <span v-if="count > 1">
          {{ count }} photos<span v-if="anyMixed"> · Mixed</span>
        </span>
        <span v-else>{{ source }}</span>
      </span>
    </div>

    <div class="space-y-2.5">
      <div class="flex items-center justify-between gap-2">
        <span class="w-14 shrink-0 text-2xs text-subtle">Rating</span>
        <div class="flex items-center gap-0.5" role="group" aria-label="Star rating">
          <button
            v-for="value in [1, 2, 3, 4, 5]"
            :key="value"
            class="grid h-7 w-7 place-items-center rounded-md transition hover:bg-panel-2 disabled:opacity-50"
            :class="starFilled(value) ? 'text-amber-400' : 'text-subtle/60'"
            :title="
              !rating.mixed && rating.value === value
                ? `Clear ${value}-star rating`
                : `Rate ${value} ${value === 1 ? 'star' : 'stars'}`
            "
            :aria-label="`Rate ${value} ${value === 1 ? 'star' : 'stars'}`"
            :aria-pressed="starFilled(value)"
            :disabled="busy"
            @click="setRating(value)"
          >
            <Star
              class="h-4 w-4"
              :fill="starFilled(value) ? 'currentColor' : 'none'"
              :stroke-width="starFilled(value) ? 1.5 : 1.75"
            />
          </button>
          <span v-if="rating.mixed" class="ml-1 text-2xs text-subtle">Mixed</span>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <span class="w-14 shrink-0 text-2xs text-subtle">Label</span>
        <div class="flex items-center gap-1.5" role="group" aria-label="Color label">
          <button
            v-for="color in COLOR_LABELS"
            :key="color"
            class="h-5 w-5 rounded-full ring-2 ring-offset-1 ring-offset-panel transition hover:scale-110 disabled:opacity-50"
            :class="[
              COLOR_LABEL_SWATCHES[color],
              labelActive(color)
                ? ['ring-ink/70', 'scale-110']
                : ['ring-transparent', 'opacity-75 hover:opacity-100'],
            ]"
            :title="
              labelActive(color)
                ? `Clear ${COLOR_LABEL_NAMES[color].toLowerCase()} label`
                : `Set ${COLOR_LABEL_NAMES[color].toLowerCase()} label`
            "
            :aria-label="`Set ${COLOR_LABEL_NAMES[color]} label`"
            :aria-pressed="labelActive(color)"
            :disabled="busy"
            @click="setLabel(color)"
          />
          <span v-if="label.mixed" class="ml-0.5 text-2xs text-subtle">Mixed</span>
          <span
            v-else-if="label.value"
            class="ml-0.5 text-2xs text-muted"
          >
            {{ COLOR_LABEL_NAMES[label.value] }}
          </span>
          <span v-else class="ml-0.5 text-2xs text-subtle">None</span>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <span class="w-14 shrink-0 text-2xs text-subtle">Culling</span>
        <div class="flex items-center gap-1" role="group" aria-label="Culling flag">
          <button
            class="flex h-7 items-center gap-1 rounded-md border px-2 text-2xs font-medium transition disabled:opacity-50"
            :class="
              cullActive('pick')
                ? 'border-success/40 bg-success/15 text-success'
                : 'border-line bg-panel text-muted hover:bg-panel-2 hover:text-ink'
            "
            title="Pick"
            :aria-pressed="cullActive('pick')"
            :disabled="busy"
            @click="setCull('pick')"
          >
            <Flag class="h-3 w-3" :fill="cullActive('pick') ? 'currentColor' : 'none'" />
            Pick
          </button>
          <button
            class="flex h-7 items-center gap-1 rounded-md border px-2 text-2xs font-medium transition disabled:opacity-50"
            :class="
              cullActive('reject')
                ? 'border-danger/40 bg-danger/15 text-danger'
                : 'border-line bg-panel text-muted hover:bg-panel-2 hover:text-ink'
            "
            title="Reject"
            :aria-pressed="cullActive('reject')"
            :disabled="busy"
            @click="setCull('reject')"
          >
            <X class="h-3 w-3" />
            Reject
          </button>
          <button
            class="grid h-7 w-7 place-items-center rounded-md border border-line bg-panel text-subtle transition hover:bg-panel-2 hover:text-ink disabled:opacity-50"
            title="Remove cull flag"
            :disabled="busy"
            @click="unflag"
          >
            <CircleSlash class="h-3 w-3" />
          </button>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <span class="w-14 shrink-0 text-2xs text-subtle">Favorite</span>
        <button
          class="flex h-7 items-center gap-1.5 rounded-md border px-2 text-2xs font-medium transition disabled:opacity-50"
          :class="
            !favorite.mixed && favorite.value
              ? 'border-rose-400/40 bg-rose-400/15 text-rose-400'
              : 'border-line bg-panel text-muted hover:bg-panel-2 hover:text-ink'
          "
          :title="
            !favorite.mixed && favorite.value
              ? `Remove favorite (${store.shortcutLabel('metadata.favorite')})`
              : `Mark as favorite (${store.shortcutLabel('metadata.favorite')})`
          "
          :aria-pressed="!favorite.mixed && favorite.value"
          :disabled="busy"
          @click="toggleFavorite"
        >
          <Heart
            class="h-3 w-3"
            :fill="!favorite.mixed && favorite.value ? 'currentColor' : 'none'"
          />
          {{ favorite.mixed ? 'Mixed' : favorite.value ? 'Favorite' : 'Not favorite' }}
        </button>
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
  </section>
</template>
