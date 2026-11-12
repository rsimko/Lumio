<script setup lang="ts">
import { MapPin } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import GeoMapPreview from '@/components/GeoMapPreview.vue';
import InfoSection from '@/components/InfoSection.vue';
import MetadataField from '@/components/MetadataField.vue';
import MetadataSavedBadge from '@/components/MetadataSavedBadge.vue';
import {
  formatAltitude,
  formatCoordinates,
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

const latitudeValue = computed(() =>
  latitude.value.mixed ? '' : formatLatitude(latitude.value.value),
);
const longitudeValue = computed(() =>
  longitude.value.mixed ? '' : formatLongitude(longitude.value.value),
);
const altitudeValue = computed(() =>
  altitude.value.mixed || altitude.value.value === null ? '' : String(altitude.value.value),
);

const cameraAvailable = computed(
  () => props.cameraLatitude !== null && props.cameraLongitude !== null,
);
const cameraCoordinates = computed(() =>
  formatCoordinates(props.cameraLatitude, props.cameraLongitude),
);
const cameraAltitudeLabel = computed(() => formatAltitude(props.cameraAltitude));

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
    apply({ clearLatitude: true });
    return;
  }
  const parsed = parseLatitude(raw);
  if (parsed === null) {
    latitudeError.value = 'Enter a latitude between -90 and 90, e.g. 48.8567 N.';
    return;
  }
  apply({ latitude: parsed });
}

function commitLongitude(raw: string): void {
  longitudeError.value = '';
  if (!raw) {
    apply({ clearLongitude: true });
    return;
  }
  const parsed = parseLongitude(raw);
  if (parsed === null) {
    longitudeError.value = 'Enter a longitude between -180 and 180, e.g. 2.3508 E.';
    return;
  }
  apply({ longitude: parsed });
}

function commitAltitude(raw: string): void {
  altitudeError.value = '';
  if (!raw) {
    apply({ clearAltitude: true });
    return;
  }
  const parsed = parseAltitude(raw);
  if (parsed === null) {
    altitudeError.value = 'Enter an altitude in metres, for example 125 or -12.5.';
    return;
  }
  apply({ altitude: parsed });
}

function useCameraGps(): void {
  if (!cameraAvailable.value) return;
  const iptc: IptcUpdate = {
    latitude: props.cameraLatitude as number,
    longitude: props.cameraLongitude as number,
  };
  if (props.cameraAltitude !== null) iptc.altitude = props.cameraAltitude;
  apply(iptc);
}
</script>

<template>
  <InfoSection title="Location &amp; geolocation">
    <template #actions>
      <MetadataSavedBadge />
    </template>

    <div class="space-y-2.5">
      <GeoMapPreview :latitude="latitude.value" :longitude="longitude.value" />

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

      <div class="border-t border-line/60 pt-2.5">
        <div class="mb-1.5 flex items-center justify-between">
          <span class="text-2xs font-medium text-ink">Coordinates</span>
          <span class="text-[10px] text-subtle">Decimal or DMS</span>
        </div>
        <div class="grid grid-cols-2 gap-2">
          <MetadataField
            label="Latitude"
            :value="latitudeValue"
            :mixed="latitude.mixed"
            placeholder="48.8567 N"
            inputmode="decimal"
            :invalid="Boolean(latitudeError)"
            @commit="commitLatitude"
          />
          <MetadataField
            label="Longitude"
            :value="longitudeValue"
            :mixed="longitude.mixed"
            placeholder="2.3508 E"
            inputmode="decimal"
            :invalid="Boolean(longitudeError)"
            @commit="commitLongitude"
          />
        </div>
        <p v-if="latitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ latitudeError }}
        </p>
        <p v-if="longitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ longitudeError }}
        </p>
        <MetadataField
          class="mt-2"
          label="Altitude"
          :value="altitudeValue"
          :mixed="altitude.mixed"
          placeholder="Metres above sea level"
          inputmode="decimal"
          :hint="altitude.value !== null && !altitude.mixed ? 'metres' : ''"
          :invalid="Boolean(altitudeError)"
          @commit="commitAltitude"
        />
        <p v-if="altitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ altitudeError }}
        </p>
      </div>

      <div
        v-if="cameraAvailable"
        class="rounded-md border border-line bg-panel/60 px-2 py-1.5"
      >
        <div class="flex items-center gap-1.5">
          <MapPin class="h-3 w-3 shrink-0 text-subtle" />
          <span class="min-w-0 flex-1 truncate text-2xs text-subtle" :title="cameraCoordinates">
            Camera GPS: <span class="text-muted">{{ cameraCoordinates }}</span>
            <span v-if="cameraAltitudeLabel" class="text-subtle">
              · {{ cameraAltitudeLabel }}
            </span>
          </span>
        </div>
        <button class="mt-1.5 w-full btn justify-center px-2" @click="useCameraGps">
          Use camera GPS
        </button>
      </div>
    </div>
  </InfoSection>
</template>
