<script setup lang="ts">
import { MapPin } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

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

const cameraAvailable = computed(
  () => props.cameraLatitude !== null && props.cameraLongitude !== null,
);
const cameraCoordinates = computed(() =>
  formatCoordinates(props.cameraLatitude, props.cameraLongitude),
);
const cameraAltitudeLabel = computed(() => formatAltitude(props.cameraAltitude));

/**
 * Camera EXIF GPS is the preferred source when present. Choosing XMP in the
 * source switch, or editing a coordinate, shows and edits the stored XMP
 * values instead.
 */
const coordinateSource = ref<'camera' | 'xmp'>('xmp');

watch(
  [() => selection.value.map((asset) => asset.id).join('|'), cameraAvailable],
  () => {
    coordinateSource.value = cameraAvailable.value ? 'camera' : 'xmp';
  },
  { immediate: true },
);

const cameraActive = computed(
  () => cameraAvailable.value && coordinateSource.value === 'camera',
);
const effectiveLatitude = computed(() =>
  cameraActive.value ? props.cameraLatitude : latitude.value.value,
);
const effectiveLongitude = computed(() =>
  cameraActive.value ? props.cameraLongitude : longitude.value.value,
);
const effectiveAltitude = computed(() =>
  cameraActive.value ? props.cameraAltitude : altitude.value.value,
);
const latitudeMixed = computed(() => (cameraActive.value ? false : latitude.value.mixed));
const longitudeMixed = computed(() => (cameraActive.value ? false : longitude.value.mixed));
const altitudeMixed = computed(() => (cameraActive.value ? false : altitude.value.mixed));

const latitudeFieldValue = computed(() =>
  latitudeMixed.value ? '' : formatLatitude(effectiveLatitude.value),
);
const longitudeFieldValue = computed(() =>
  longitudeMixed.value ? '' : formatLongitude(effectiveLongitude.value),
);
const altitudeFieldValue = computed(() =>
  altitudeMixed.value || effectiveAltitude.value === null ? '' : String(effectiveAltitude.value),
);

function nearlyEqual(left: number | null, right: number | null): boolean {
  return left !== null && right !== null && Math.abs(left - right) < 1e-5;
}

const cameraDiffers = computed(() => {
  if (!cameraAvailable.value) return false;
  if (!nearlyEqual(latitude.value.value, props.cameraLatitude)) return true;
  if (!nearlyEqual(longitude.value.value, props.cameraLongitude)) return true;
  if (
    props.cameraAltitude !== null &&
    !nearlyEqual(altitude.value.value, props.cameraAltitude)
  ) {
    return true;
  }
  return false;
});

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
    coordinateSource.value = 'xmp';
    apply({ clearLatitude: true });
    return;
  }
  const parsed = parseLatitude(raw);
  if (parsed === null) {
    latitudeError.value = 'Enter a latitude between -90 and 90, e.g. 48.8567 N.';
    return;
  }
  coordinateSource.value = 'xmp';
  apply({ latitude: parsed });
}

function commitLongitude(raw: string): void {
  longitudeError.value = '';
  if (!raw) {
    coordinateSource.value = 'xmp';
    apply({ clearLongitude: true });
    return;
  }
  const parsed = parseLongitude(raw);
  if (parsed === null) {
    longitudeError.value = 'Enter a longitude between -180 and 180, e.g. 2.3508 E.';
    return;
  }
  coordinateSource.value = 'xmp';
  apply({ longitude: parsed });
}

function commitAltitude(raw: string): void {
  altitudeError.value = '';
  if (!raw) {
    coordinateSource.value = 'xmp';
    apply({ clearAltitude: true });
    return;
  }
  const parsed = parseAltitude(raw);
  if (parsed === null) {
    altitudeError.value = 'Enter an altitude in metres, for example 125 or -12.5.';
    return;
  }
  coordinateSource.value = 'xmp';
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
  coordinateSource.value = 'camera';
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

      <div class="border-t border-line/60 pt-2.5">
        <div class="mb-1.5 flex items-center justify-between gap-2">
          <span class="text-2xs font-medium text-ink">Coordinates</span>
          <div
            v-if="cameraAvailable"
            class="inline-flex items-center gap-0.5 rounded-md border border-line bg-app/60 p-0.5"
            role="group"
            aria-label="Coordinate source"
          >
            <button
              type="button"
              class="rounded px-1.5 py-0.5 text-[10px] font-semibold transition"
              :class="
                cameraActive
                  ? 'bg-accent-soft text-accent'
                  : 'text-subtle hover:bg-panel-2/60 hover:text-ink'
              "
              :aria-pressed="cameraActive"
              @click="coordinateSource = 'camera'"
            >
              Camera
            </button>
            <button
              type="button"
              class="rounded px-1.5 py-0.5 text-[10px] font-semibold transition"
              :class="
                !cameraActive
                  ? 'bg-accent-soft text-accent'
                  : 'text-subtle hover:bg-panel-2/60 hover:text-ink'
              "
              :aria-pressed="!cameraActive"
              @click="coordinateSource = 'xmp'"
            >
              XMP
            </button>
          </div>
          <span v-else class="text-[10px] text-subtle">Decimal or DMS</span>
        </div>

        <div v-if="cameraAvailable" class="mb-2 rounded-md border border-line bg-panel/60 px-2 py-1.5">
          <div class="flex items-center gap-1.5">
            <MapPin class="h-3 w-3 shrink-0 text-subtle" />
            <span class="min-w-0 flex-1 truncate text-2xs text-subtle" :title="cameraCoordinates">
              Camera GPS: <span class="text-muted">{{ cameraCoordinates }}</span>
              <span v-if="cameraAltitudeLabel" class="text-subtle">
                · {{ cameraAltitudeLabel }}
              </span>
            </span>
            <span
              v-if="cameraActive"
              class="shrink-0 rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent"
            >
              In use
            </span>
          </div>
          <p v-if="cameraActive" class="mt-1 text-[10px] leading-relaxed text-subtle">
            Editing coordinates stores XMP values and switches the view to XMP.
          </p>
          <button
            v-else-if="cameraDiffers"
            type="button"
            class="mt-1.5 w-full btn justify-center px-2"
            @click="useCameraGps"
          >
            Use camera GPS
          </button>
        </div>

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
        <p v-if="latitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ latitudeError }}
        </p>
        <p v-if="longitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ longitudeError }}
        </p>
        <MetadataField
          class="mt-2"
          label="Altitude"
          :value="altitudeFieldValue"
          :mixed="altitudeMixed"
          placeholder="Metres above sea level"
          inputmode="decimal"
          :hint="altitudeFieldValue ? 'metres' : ''"
          :invalid="Boolean(altitudeError)"
          @commit="commitAltitude"
        />
        <p v-if="altitudeError" class="mt-1 text-2xs leading-relaxed text-danger">
          {{ altitudeError }}
        </p>
      </div>
    </div>
  </InfoSection>
</template>
