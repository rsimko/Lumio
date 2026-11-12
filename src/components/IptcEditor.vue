<script setup lang="ts">
import { computed } from 'vue';

import InfoSection from '@/components/InfoSection.vue';
import MetadataField from '@/components/MetadataField.vue';
import MetadataSavedBadge from '@/components/MetadataSavedBadge.vue';
import { commonMetadataValue } from '@/lib/metadata';
import { useLibraryStore } from '@/stores/library';
import type { IptcUpdate, PhotoMetadata } from '@/types';

type TextField =
  | 'title'
  | 'headline'
  | 'description'
  | 'creator'
  | 'credit'
  | 'copyright'
  | 'source';

interface FieldDefinition {
  key: TextField;
  label: string;
  placeholder: string;
  multiline?: boolean;
  maxlength?: number;
}

const fields: FieldDefinition[] = [
  { key: 'title', label: 'Title', placeholder: 'Short title', maxlength: 255 },
  { key: 'headline', label: 'Headline', placeholder: 'Editorial headline', maxlength: 255 },
  {
    key: 'description',
    label: 'Description',
    placeholder: 'Caption or description',
    multiline: true,
  },
  { key: 'creator', label: 'Creator', placeholder: 'Photographer or author', maxlength: 255 },
  { key: 'credit', label: 'Credit', placeholder: 'Credit line', maxlength: 255 },
  { key: 'copyright', label: 'Copyright', placeholder: 'Rights notice', maxlength: 255 },
  { key: 'source', label: 'Source', placeholder: 'Agency or source', maxlength: 255 },
];

const store = useLibraryStore();
const selection = computed(() => store.selectedAssets);
const states = computed(
  () =>
    Object.fromEntries(
      fields.map((field) => [
        field.key,
        commonMetadataValue(
          selection.value,
          (metadata: PhotoMetadata) => metadata.iptc?.[field.key] ?? '',
          '',
        ),
      ]),
    ) as Record<TextField, { value: string; mixed: boolean }>,
);

function commit(key: TextField, value: string): void {
  if (!selection.value.length) return;
  const update: IptcUpdate = { [key]: value };
  void store.applyMetadataPatch({ iptc: update }, selection.value, { silent: true });
}
</script>

<template>
  <InfoSection title="IPTC">
    <template #actions>
      <MetadataSavedBadge />
    </template>

    <div class="space-y-2">
      <div v-if="selection.length > 1" class="text-2xs leading-relaxed text-subtle">
        IPTC changes apply to all {{ selection.length }} selected photos.
      </div>

      <MetadataField
        v-for="field in fields"
        :key="field.key"
        :label="field.label"
        :value="states[field.key].value"
        :mixed="states[field.key].mixed"
        :placeholder="field.placeholder"
        :multiline="field.multiline"
        :maxlength="field.maxlength"
        @commit="(value) => commit(field.key, value)"
      />
    </div>
  </InfoSection>
</template>
