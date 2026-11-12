import { ref } from 'vue';

import { api } from '@/api';

const fonts = ref<string[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
let loadPromise: Promise<void> | null = null;

function ensureLoaded(): Promise<void> {
  if (!loadPromise) {
    loading.value = true;
    loadPromise = api
      .listSystemFonts()
      .then((families) => {
        fonts.value = families;
        error.value = null;
      })
      .catch((reason: unknown) => {
        error.value = reason instanceof Error ? reason.message : String(reason);
      })
      .finally(() => {
        loading.value = false;
      });
  }
  return loadPromise;
}

/** Shared, one-shot list of installed font families for the settings dialog. */
export function useSystemFonts() {
  void ensureLoaded();
  return { fonts, loading, error };
}
