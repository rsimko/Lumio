<script setup lang="ts">
import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  HardDrive,
  Loader2,
  RefreshCw,
  X,
} from 'lucide-vue-next';
import { open as openPathDialog } from '@tauri-apps/plugin-dialog';
import { computed, onMounted, ref } from 'vue';

import SettingsSection from '@/components/settings/SettingsSection.vue';
import { api, errorMessage } from '@/api';
import { normalizeRenamePostfix, renamePostfixExample, xmpNamingLabel } from '@/lib/ingest';
import { useLibraryStore } from '@/stores/library';
import type { IngestSettings, IngestTools } from '@/types';

const store = useLibraryStore();

function cloneIngestSettings(settings: IngestSettings): IngestSettings {
  return {
    ...settings,
    templates: settings.templates.map((template) => ({ ...template })),
    iptc: { ...settings.iptc, keywords: [...settings.iptc.keywords] },
  };
}

const ingestDraft = ref<IngestSettings>(cloneIngestSettings(store.ingestSettings));
const ingestTools = ref<IngestTools | null>(null);
const detectingIngestTools = ref(false);
const savingIngest = ref(false);
const ingestMessage = ref<string | null>(null);
const ingestError = ref<string | null>(null);

const renamePreview = computed(() => renamePostfixExample(ingestDraft.value.renamePostfix));
const renamePostfixHint = computed(() => {
  const raw = ingestDraft.value.renamePostfix;
  if (/[/\\:]/.test(raw)) return 'Characters /, \\ and : are removed when the settings are saved.';
  if (!raw.includes('#')) return 'A # placeholder is appended automatically when saved.';
  return null;
});

function normalizeIngestPostfix(): void {
  ingestDraft.value.renamePostfix = normalizeRenamePostfix(ingestDraft.value.renamePostfix);
}

async function chooseDefaultIngestSource(): Promise<void> {
  const picked = await openPathDialog({
    directory: true,
    multiple: false,
    title: 'Choose the default ingest source',
  });
  if (!picked || Array.isArray(picked)) return;
  ingestDraft.value.defaultSource = picked;
}

async function detectIngestTools(): Promise<void> {
  detectingIngestTools.value = true;
  try {
    ingestTools.value = await api.detectIngestTools(ingestDraft.value.exiftoolPath);
  } catch (error) {
    ingestError.value = errorMessage(error);
  } finally {
    detectingIngestTools.value = false;
  }
}

async function saveIngestSettings(): Promise<void> {
  savingIngest.value = true;
  ingestError.value = null;
  ingestMessage.value = null;
  try {
    ingestDraft.value.defaultSource = ingestDraft.value.defaultSource?.trim() || null;
    normalizeIngestPostfix();
    ingestDraft.value = cloneIngestSettings(await store.saveIngestSettings(ingestDraft.value));
    ingestMessage.value = 'Ingest settings saved.';
    await detectIngestTools();
  } catch (error) {
    ingestError.value = errorMessage(error);
  } finally {
    savingIngest.value = false;
  }
}

onMounted(() => {
  void detectIngestTools();
});
</script>

<template>
  <SettingsSection
    title="Ingest settings"
    description="Defaults for copying camera cards and folders into a structured destination."
  >
    <template #actions>
      <div class="flex items-center gap-2">
        <span v-if="ingestMessage" class="text-2xs text-success">{{ ingestMessage }}</span>
        <span v-if="ingestError" class="max-w-[220px] text-2xs text-danger">
          {{ ingestError }}
        </span>
        <button class="btn text-2xs" :disabled="savingIngest" @click="saveIngestSettings()">
          <Loader2 v-if="savingIngest" class="h-3 w-3 animate-spin" />
          <HardDrive v-else class="h-3 w-3" />
          Save settings
        </button>
      </div>
    </template>

    <div class="divide-y divide-line">
      <div class="px-3.5 py-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <p class="text-xs font-medium text-ink">ExifTool</p>
            <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
              Reads capture dates and is required for IPTC and keyword writing.
            </p>
          </div>
          <span
            class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-2xs"
            :class="
              ingestTools?.exiftool
                ? 'border-success/30 bg-success/10 text-success'
                : 'border-warning/40 bg-warning/10 text-warning'
            "
          >
            <CheckCircle2 v-if="ingestTools?.exiftool" class="h-3 w-3" />
            <AlertTriangle v-else class="h-3 w-3" />
            {{
              detectingIngestTools
                ? 'Checking…'
                : ingestTools?.exiftool
                  ? `ExifTool ${ingestTools.exiftool.version}`
                  : 'Not detected'
            }}
          </span>
        </div>
        <div class="mt-2 flex gap-1.5">
          <input
            v-model="ingestDraft.exiftoolPath"
            class="h-7 min-w-0 flex-1 rounded border border-line bg-panel px-2 font-mono text-2xs text-ink outline-none focus:border-accent/70"
            placeholder="Auto-detect from PATH and Homebrew folders"
            spellcheck="false"
          />
          <button class="btn h-7 text-2xs" :disabled="detectingIngestTools" @click="detectIngestTools()">
            <Loader2 v-if="detectingIngestTools" class="h-3 w-3 animate-spin" />
            <RefreshCw v-else class="h-3 w-3" />
            Detect
          </button>
        </div>
        <p v-if="ingestTools?.exiftool" class="mt-1 break-all font-mono text-3xs text-subtle">
          {{ ingestTools.exiftool.path }}
        </p>
      </div>

      <div class="px-3.5 py-3">
        <p class="text-xs font-medium text-ink">Default ingest source</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          Pre-filled when you open Ingest photos. You can still change or clear it in the dialog.
        </p>
        <div class="mt-2 flex gap-1.5">
          <input
            v-model="ingestDraft.defaultSource"
            class="h-7 min-w-0 flex-1 rounded border border-line bg-panel px-2 font-mono text-2xs text-ink outline-none focus:border-accent/70"
            placeholder="No default source"
            spellcheck="false"
            aria-label="Default ingest source"
          />
          <button class="btn h-7 shrink-0 text-2xs" @click="chooseDefaultIngestSource()">
            <FolderOpen class="h-3 w-3" />
            Choose
          </button>
          <button
            v-if="ingestDraft.defaultSource"
            class="btn-icon-sm shrink-0"
            title="Clear the default ingest source"
            @click="ingestDraft.defaultSource = null"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <div class="px-3.5 py-3">
        <p class="text-xs font-medium text-ink">Conflict rename postfix</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          Appended when a destination name already exists or Keep both renames a photo.
          <span class="font-mono">#</span> is replaced by the number.
        </p>
        <div class="mt-2 flex flex-wrap items-center gap-2">
          <input
            v-model="ingestDraft.renamePostfix"
            class="h-7 w-28 rounded border border-line bg-panel px-2 font-mono text-2xs text-ink outline-none focus:border-accent/70"
            placeholder="_#"
            spellcheck="false"
            aria-label="Conflict rename postfix"
            @blur="normalizeIngestPostfix"
          />
          <span class="text-2xs text-subtle">
            Example
            <span class="font-mono text-muted">{{ renamePreview }}</span>
          </span>
        </div>
        <p v-if="renamePostfixHint" class="mt-1 text-2xs text-subtle">{{ renamePostfixHint }}</p>
      </div>

      <div class="px-3.5 py-3">
        <p class="text-xs font-medium text-ink">XMP sidecar naming</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          Used when copying a RAW photo and its XMP sidecar.
        </p>
        <select
          v-model="ingestDraft.xmpNaming"
          class="mt-2 h-7 w-full rounded border border-line bg-panel px-2 font-mono text-2xs text-ink outline-none focus:border-accent/70"
        >
          <option value="filenameExt">{{ xmpNamingLabel('filenameExt') }}</option>
          <option value="filename">{{ xmpNamingLabel('filename') }}</option>
        </select>
      </div>

      <div class="flex items-center justify-between gap-4 px-3.5 py-3">
        <div>
          <p class="text-xs font-medium text-ink">Add destination to library</p>
          <p class="mt-0.5 text-2xs text-subtle">Default for the checkbox in the ingest dialog.</p>
        </div>
        <button
          class="switch"
          :class="ingestDraft.addToLibrary ? 'switch-on' : ''"
          role="switch"
          :aria-checked="ingestDraft.addToLibrary"
          aria-label="Add destination to library"
          @click="ingestDraft.addToLibrary = !ingestDraft.addToLibrary"
        >
          <span class="switch-knob" />
        </button>
      </div>

      <div class="px-3.5 py-3">
        <p class="text-xs font-medium text-ink">Saved templates</p>
        <p class="mt-0.5 text-2xs leading-relaxed text-subtle">
          {{ ingestDraft.templates.length }}
          {{ ingestDraft.templates.length === 1 ? 'named template' : 'named templates' }}. Manage
          them while ingesting.
        </p>
      </div>
    </div>
  </SettingsSection>
</template>
