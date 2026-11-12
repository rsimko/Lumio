<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import GeoMapPreview from '@/components/GeoMapPreview.vue';
import InfoSection from '@/components/InfoSection.vue';
import MetadataField from '@/components/MetadataField.vue';
import MetadataSavedBadge from '@/components/MetadataSavedBadge.vue';
import {
  formatLatitude,
  formatLongitude,
  parseAltitude,
  parseLatitude,
  parseLongitude,
} from '@/lib/geo';
import { commonMetadataValue } from '@/lib/metadata';
import { useLibraryStore } from '@/stores/library';
import type { IptcUpdate, PhotoMetadata } from '@/types';

type LocationField = 'city' | 'state' | 'country' | 'countryCode' | 'location';

interface FieldDefinition {
  key: LocationField;
  label: string;
  placeholder: string;
  maxlength?: number;
}

const fields: FieldDefinition[] = [
  { key: 'city', label: 'City', placeholder: 'City', maxlength: 255 },
  { key: 'state', label: 'State / Province', placeholder: 'State or province', maxlength: 255 },
  { key: 'country', label: 'Country', placeholder: 'Country name', maxlength: 255 },
  { key: 'countryCode', label: 'Country code', placeholder: 'e.g. DE', maxlength: 3 },
  { key: 'location', label: 'Location', placeholder: 'Sublocation or landmark', maxlength: 255 },
];

const props = withDefaults(
  defineProps<{
    cameraLatitude?: number | null;
    cameraLongitude?: number | null;
    cameraAltitude?: number | null;
  }>(),
  {
    cameraLatitude: null,
    cameraLongitude: null,
    cameraAltitude: null,
  },
);

const store = useLibraryStore();
const selection = computed(() => store.selectedAssets);
const latitudeError = ref('');
const longitudeError = ref('');
const altitudeError = ref('');

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
    ) as Record<LocationField, { value: string; mixed: boolean }>,
);

const latitude = computed(() =>
  commonMetadataValue<number | null>(
    selection.value,
    (metadata) => metadata.iptc?.latitude ?? null,
    null,
  ),
);
const longitude = computed(() =>
  commonMetadataValue<number | null>(
    selection.value,
    (metadata) => metadata.iptc?.longitude ?? null,
    null,
  ),
);
const altitude = computed(() =>
  commonMetadataValue<number | null>(
    selection.value,
    (metadata) => metadata.iptc?.altitude ?? null,
    null,
  ),
);

const cameraCoordinatesAvailable = computed(
  () => props.cameraLatitude !== null && props.cameraLongitude !== null,
);
const cameraAltitudeAvailable = computed(() => props.cameraAltitude !== null);

/**
 * Camera EXIF GPS is shown by default when present, with XMP values as the
 * fallback. Editing a field stores the value in XMP and switches only that
 * field to the stored XMP value.
 */
const latitudeSource = ref<'camera' | 'xmp'>('xmp');
const longitudeSource = ref<'camera' | 'xmp'>('xmp');
const altitudeSource = ref<'camera' | 'xmp'>('xmp');

watch(
  [
    () => selection.value.map((asset) => asset.id).join('|'),
    cameraCoordinatesAvailable,
    cameraAltitudeAvailable,
  ],
  () => {
    const coordinates = cameraCoordinatesAvailable.value ? 'camera' : 'xmp';
    latitudeSource.value = coordinates;
    longitudeSource.value = coordinates;
    altitudeSource.value = cameraAltitudeAvailable.value ? 'camera' : 'xmp';
  },
  { immediate: true },
);

const cameraLatitudeActive = computed(
  () => cameraCoordinatesAvailable.value && latitudeSource.value === 'camera',
);
const cameraLongitudeActive = computed(
  () => cameraCoordinatesAvailable.value && longitudeSource.value === 'camera',
);
const cameraAltitudeActive = computed(
  () => cameraAltitudeAvailable.value && altitudeSource.value === 'camera',
);

const effectiveLatitude = computed(() =>
  cameraLatitudeActive.value ? props.cameraLatitude : latitude.value.value,
);
const effectiveLongitude = computed(() =>
  cameraLongitudeActive.value ? props.cameraLongitude : longitude.value.value,
);
const effectiveAltitude = computed(() =>
  cameraAltitudeActive.value ? props.cameraAltitude : altitude.value.value,
);
const latitudeMixed = computed(() =>
  cameraLatitudeActive.value ? false : latitude.value.mixed,
);
const longitudeMixed = computed(() =>
  cameraLongitudeActive.value ? false : longitude.value.mixed,
);
const altitudeMixed = computed(() =>
  cameraAltitudeActive.value ? false : altitude.value.mixed,
);

const latitudeFieldValue = computed(() =>
  latitudeMixed.value ? '' : formatLatitude(effectiveLatitude.value),
);
const longitudeFieldValue = computed(() =>
  longitudeMixed.value ? '' : formatLongitude(effectiveLongitude.value),
);
const altitudeFieldValue = computed(() =>
  altitudeMixed.value || effectiveAltitude.value === null ? '' : String(effectiveAltitude.value),
);

function apply(iptc: IptcUpdate): void {
  if (!selection.value.length) return;
  void store.applyMetadataPatch({ iptc }, selection.value, { silent: true });
}

function commitField(key: LocationField, value: string): void {
  apply({ [key]: value });
}

function commitLatitude(raw: string): void {
  latitudeError.value = '';
  if (!raw) {
    latitudeSource.value = 'xmp';
    apply({ clearLatitude: true });
    return;
  }
  const parsed = parseLatitude(raw);
  if (parsed === null) {
    latitudeError.value = 'Enter a latitude between -90 and 90, e.g. 48.8567 N.';
    return;
  }
  latitudeSource.value = 'xmp';
  apply({ latitude: parsed });
}

function commitLongitude(raw: string): void {
  longitudeError.value = '';
  if (!raw) {
    longitudeSource.value = 'xmp';
    apply({ clearLongitude: true });
    return;
  }
  const parsed = parseLongitude(raw);
  if (parsed === null) {
    longitudeError.value = 'Enter a longitude between -180 and 180, e.g. 2.3508 E.';
    return;
  }
  longitudeSource.value = 'xmp';
  apply({ longitude: parsed });
}

function commitAltitude(raw: string): void {
  altitudeError.value = '';
  if (!raw) {
    altitudeSource.value = 'xmp';
    apply({ clearAltitude: true });
    return;
  }
  const parsed = parseAltitude(raw);
  if (parsed === null) {
    altitudeError.value = 'Enter an altitude in metres, for example 125 or -12.5.';
    return;
  }
  altitudeSource.value = 'xmp';
  apply({ altitude: parsed });
}
</script>

<template>
  <InfoSection title="Location &amp; geolocation">
    <template #actions>
      <MetadataSavedBadge />
    </template>

    <div class="space-y-2.5">
      <GeoMapPreview :latitude="effectiveLatitude" :longitude="effectiveLongitude" />

      <div v-if="selection.length > 1" class="text-2xs leading-relaxed text-subtle">
        Location changes apply to all {{ selection.length }} selected photos.
      </div>

      <div class="space-y-2">
        <MetadataField
          v-for="field in fields"
          :key="field.key"
          :label="field.label"
          :value="states[field.key].value"
          :mixed="states[field.key].mixed"
          :placeholder="field.placeholder"
          :maxlength="field.maxlength"
          @commit="(value) => commitField(field.key, value)"
        />
      </div>

      <div class="space-y-2 border-t border-line/60 pt-2.5">
        <div class="grid grid-cols-2 gap-2">
          <MetadataField
            label="Latitude"
            :value="latitudeFieldValue"
            :mixed="latitudeMixed"
            placeholder="48.8567 N"
            inputmode="decimal"
            :invalid="Boolean(latitudeError)"
            @commit="commitLatitude"
          />
          <MetadataField
            label="Longitude"
            :value="longitudeFieldValue"
            :mixed="longitudeMixed"
            placeholder="2.3508 E"
            inputmode="decimal"
            :invalid="Boolean(longitudeError)"
            @commit="commitLongitude"
          />
        </div>
        <p v-if="latitudeError" class="text-2xs leading-relaxed text-danger">
          {{ latitudeError }}
        </p>
        <p v-if="longitudeError" class="text-2xs leading-relaxed text-danger">
          {{ longitudeError }}
        </p>
        <MetadataField
          label="Altitude"
          :value="altitudeFieldValue"
          :mixed="altitudeMixed"
          placeholder="Metres above sea level"
          inputmode="decimal"
          :hint="altitudeFieldValue ? 'metres' : ''"
          :invalid="Boolean(altitudeError)"
          @commit="commitAltitude"
        />
        <p v-if="altitudeError" class="text-2xs leading-relaxed text-danger">
          {{ altitudeError }}
        </p>
      </div>
    </div>
  </InfoSection>
</template>
