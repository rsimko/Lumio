<script setup lang="ts">
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  CircleSlash,
  Download,
  Eye,
  FileImage,
  FileText,
  FolderOpen,
  FolderPlus,
  HardDrive,
  ListChecks,
  ListTree,
  Loader2,
  MonitorUp,
  Play,
  Plus,
  RefreshCw,
  Save,
  ScrollText,
  Table2,
  Trash2,
  X,
  XCircle,
} from 'lucide-vue-next';
import { computed, ref } from 'vue';

import AppDialog from '@/components/AppDialog.vue';
import IngestKeywordEditor from '@/components/IngestKeywordEditor.vue';
import IngestTemplateEditor from '@/components/IngestTemplateEditor.vue';
import IngestTokenPalette from '@/components/IngestTokenPalette.vue';
import { basename, formatBytes, formatCount } from '@/lib/format';
import {
  buildIngestDestinationTree,
  captureSourceLabel,
  ingestDestinationFileName,
  previewIngestDestination,
  previewSampleFromPath,
  type IngestDestinationTreeFolder,
  type IngestPreviewSample,
} from '@/lib/ingest';
import { useIngestStore, type IngestTab } from '@/stores/ingest';
import type { IngestConflictResolution, IngestEntity } from '@/types';

const store = useIngestStore();

const selectedEntityId = ref<string | null>(null);
const templatePicker = ref('');
const iptcOpen = ref(false);
const logFilter = ref<'all' | 'warning' | 'error'>('all');
type TemplateField = 'folder' | 'filename';
const activeTemplate = ref<TemplateField>('folder');
const folderEditor = ref<{ insert: (token: string) => void } | null>(null);
const filenameEditor = ref<{ insert: (token: string) => void } | null>(null);
const previewView = ref<'table' | 'tree'>('table');

const SOURCE_FILE_EXTENSIONS = new Set([
  'cr2',
  'cr3',
  'nef',
  'arw',
  'raf',
  'orf',
  'rw2',
  'dng',
  'jpg',
  'jpeg',
  'xmp',
]);

/**
 * Use the first planned photo for the live preview, or the first selected file
 * before a Dry Run. The neutral sample remains the fallback.
 */
const previewSample = computed<IngestPreviewSample | null>(() => {
  const firstEntity = store.plan?.entities[0];
  if (firstEntity) {
    const sample = previewSampleFromPath(firstEntity.primaryPath);
    if (sample) return { ...sample, stem: firstEntity.stem || sample.stem };
  }
  for (const source of store.sources) {
    if (!isFileSource(source)) continue;
    const sample = previewSampleFromPath(source);
    if (!sample || sample.extension.toLowerCase() === 'xmp') continue;
    return sample;
  }
  return null;
});

const templatePreview = computed(() =>
  previewIngestDestination(store.draft, new Date(), previewSample.value),
);
const previewPath = computed(() => {
  const destination = store.destination?.replace(/\/+$/, '') ?? '';
  const relative = templatePreview.value.relativePath;
  if (!relative) return destination || '—';
  return destination ? `${destination}/${relative}` : relative;
});
const blockedTemplateTokens = computed(() =>
  activeTemplate.value === 'filename' ? ['/'] : [],
);

const tabs = computed<{ id: IngestTab; label: string; count?: number }[]>(() => [
  { id: 'preview', label: 'Preview', count: store.plan?.summary.entities },
  { id: 'conflicts', label: 'Conflicts', count: store.plan?.summary.conflicts },
  { id: 'log', label: 'Log', count: store.logEntries.length },
  { id: 'report', label: 'Report' },
]);

const plan = computed(() => store.plan);

interface DestinationTreeRow {
  kind: 'folder' | 'photo';
  depth: number;
  name: string;
  path: string;
  photoCount: number;
  entity: IngestEntity | null;
}

function treePhotoCount(folder: IngestDestinationTreeFolder): number {
  return (
    folder.entities.length +
    folder.folders.reduce((total, child) => total + treePhotoCount(child), 0)
  );
}

const treeRows = computed<DestinationTreeRow[]>(() => {
  if (!plan.value) return [];
  const rootPath = plan.value.destination.replace(/\/+$/, '');
  const root = buildIngestDestinationTree(plan.value);
  const rows: DestinationTreeRow[] = [];
  const walk = (folder: IngestDestinationTreeFolder, depth: number): void => {
    rows.push({
      kind: 'folder',
      depth,
      name: folder.name,
      path: folder.path ? `${rootPath}/${folder.path}` : rootPath,
      photoCount: treePhotoCount(folder),
      entity: null,
    });
    for (const child of folder.folders) walk(child, depth + 1);
    for (const entity of folder.entities) {
      rows.push({
        kind: 'photo',
        depth: depth + 1,
        name: ingestDestinationFileName(entity),
        path: entity.destinationDir,
        photoCount: 0,
        entity,
      });
    }
  };
  walk(root, 0);
  return rows;
});

function togglePreviewEntity(row: DestinationTreeRow): void {
  if (!row.entity) return;
  selectedEntityId.value =
    selectedEntityId.value === row.entity.id ? null : row.entity.id;
}

const selectedEntity = computed<IngestEntity | null>(() =>
  selectedEntityId.value ? (store.entityFor(selectedEntityId.value) ?? null) : null,
);
const filteredLog = computed(() =>
  store.logEntries.filter(
    (entry) => logFilter.value === 'all' || entry.level === logFilter.value,
  ),
);
const hasConflicts = computed(() => (plan.value?.summary.conflicts ?? 0) > 0);
const selectedTemplate = computed(() =>
  store.draft.templates.find((template) => template.id === store.selectedTemplateId),
);

function sourceName(path: string): string {
  return basename(path);
}

function isFileSource(path: string): boolean {
  const name = basename(path).toLowerCase();
  const dot = name.lastIndexOf('.');
  return dot > 0 && SOURCE_FILE_EXTENSIONS.has(name.slice(dot + 1));
}

function insertTemplateToken(token: string): void {
  const editor = activeTemplate.value === 'filename' ? filenameEditor.value : folderEditor.value;
  editor?.insert(token);
}

function relativeDestination(entity: IngestEntity): string {
  if (!plan.value) return entity.destinationStem;
  const root = plan.value.destination.replace(/\/+$/, '');
  const dir = entity.destinationDir.startsWith(root)
    ? entity.destinationDir.slice(root.length).replace(/^\/+/, '')
    : entity.destinationDir;
  return dir ? `${dir}/${entity.destinationStem}` : entity.destinationStem;
}

function statusLabel(entity: IngestEntity): string {
  if (entity.status === 'error') return 'Error';
  if (entity.status === 'conflict') return 'Conflict';
  if (entity.renamed) return 'Ready, renamed';
  return 'Ready';
}

function statusClass(entity: IngestEntity): string {
  if (entity.status === 'error') return 'border-danger/30 bg-danger/10 text-danger';
  if (entity.status === 'conflict') return 'border-warning/30 bg-warning/10 text-warning';
  if (entity.renamed) return 'border-warning/30 bg-warning/10 text-warning';
  return 'border-success/30 bg-success/10 text-success';
}

function roleLabel(role: string): string {
  if (role === 'raw') return 'RAW';
  if (role === 'jpeg') return 'JPEG';
  return 'XMP';
}

function chooseTemplate(): void {
  if (!templatePicker.value) return;
  store.applyTemplate(templatePicker.value);
  store.templateName = selectedTemplate.value?.name ?? '';
  templatePicker.value = '';
}

function updateTemplate(): void {
  const id = store.selectedTemplateId;
  if (!id) return;
  void store.saveTemplate(store.templateName || selectedTemplate.value?.name || '', id);
}

function saveAsTemplate(): void {
  void store.saveTemplate(store.templateName || 'Untitled template');
}

function deleteSelectedTemplate(): void {
  const id = store.selectedTemplateId;
  if (!id) return;
  if (!window.confirm(`Delete the template "${selectedTemplate.value?.name ?? ''}"?`)) return;
  void store.deleteTemplate(id);
}

function resolution(entity: IngestEntity, value: IngestConflictResolution): void {
  if (value === 'replace') {
    const confirmed = window.confirm(
      `Replace the existing destination files for ${entity.displayName}?\n\n${entity.conflict?.existing.join(
        '\n',
      )}\n\nExisting files are backed up while the copy is committed.`,
    );
    if (!confirmed) return;
  }
  void store.setResolution(entity.id, value);
}

function applyResolutionToAll(value: IngestConflictResolution): void {
  const entities = store.conflictEntities;
  if (!entities.length) return;
  if (value === 'replace') {
    const confirmed = window.confirm(
      `Replace existing destination files for ${entities.length} photo(s)?\n\nExisting files are backed up while each copy is committed.`,
    );
    if (!confirmed) return;
  }
  for (const entity of entities) void store.setResolution(entity.id, value);
}

function chooseSourceFolders(): void {
  void store.addSourceFolders();
}

function chooseSourceFiles(): void {
  void store.addSourceFiles();
}

function chooseDestination(): void {
  void store.chooseDestination();
}
</script>

<template>
  <AppDialog
    title="Ingest photos"
    full
    flush
    @close="store.closeDialog()"
  >
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-4 py-3">
        <div
          v-if="store.planError || store.runError"
          class="mb-3 flex items-start gap-2 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs leading-relaxed text-danger"
        >
          <AlertTriangle class="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span class="min-w-0 flex-1">{{ store.planError || store.runError }}</span>
          <button class="text-danger/70 hover:text-danger" @click="store.planError = null; store.runError = null">
            <X class="h-3 w-3" />
          </button>
        </div>

        <div class="grid gap-3 lg:grid-cols-[minmax(300px,0.9fr)_minmax(380px,1.1fr)]">
          <section class="rounded-lg border border-line bg-panel/60 p-3">
            <div class="mb-2 flex items-center justify-between gap-2">
              <h3 class="text-xs font-semibold text-ink">Sources</h3>
              <div class="flex items-center gap-1">
                <button class="btn h-7 px-2 text-[11px]" @click="chooseSourceFolders">
                  <FolderPlus class="h-3 w-3" />
                  Add folders
                </button>
                <button class="btn h-7 px-2 text-[11px]" @click="chooseSourceFiles">
                  <Plus class="h-3 w-3" />
                  Add files
                </button>
                <button
                  v-if="store.sources.length"
                  class="btn-icon-sm"
                  title="Clear sources"
                  @click="store.clearSources()"
                >
                  <X class="h-3 w-3" />
                </button>
              </div>
            </div>
            <ul
              v-if="store.sources.length"
              class="max-h-36 space-y-1 overflow-y-auto pr-1"
            >
              <li
                v-for="source in store.sources"
                :key="source"
                class="flex items-center gap-2 rounded border border-line/70 bg-elevated px-2 py-1"
              >
                <FileImage v-if="isFileSource(source)" class="h-3 w-3 shrink-0 text-subtle" />
                <FolderOpen v-else class="h-3 w-3 shrink-0 text-subtle" />
                <span
                  class="min-w-0 flex-1 truncate font-mono text-[10px] text-muted"
                  :title="source"
                >
                  {{ source }}
                </span>
                <button
                  class="text-subtle transition hover:text-danger"
                  :title="`Remove ${source}`"
                  @click="store.removeSource(source)"
                >
                  <X class="h-3 w-3" />
                </button>
              </li>
            </ul>
            <div
              v-else
              class="rounded border border-dashed border-line px-3 py-4 text-center text-[11px] leading-relaxed text-subtle"
            >
              <p class="font-medium text-muted">Choose source folders and individual photos to begin.</p>
              <p class="mt-1 text-[10px]">
                Folders are scanned recursively. A selected file brings its same-name RAW, JPEG and
                XMP companions along.
              </p>
            </div>
          </section>

          <section class="rounded-lg border border-line bg-panel/60 p-3">
            <div class="mb-2 flex items-center justify-between gap-2">
              <h3 class="text-xs font-semibold text-ink">Destination</h3>
              <button class="btn h-7 px-2 text-[11px]" @click="chooseDestination">
                <HardDrive class="h-3 w-3" />
                Choose folder
              </button>
            </div>
            <button
              class="flex w-full items-center gap-2 rounded border px-2.5 py-2 text-left transition"
              :class="
                store.destination
                  ? 'border-line bg-elevated hover:border-line-strong'
                  : 'border-dashed border-line text-subtle hover:border-line-strong hover:text-muted'
              "
              @click="chooseDestination"
            >
              <HardDrive class="h-3.5 w-3.5 shrink-0" />
              <span v-if="store.destination" class="min-w-0 flex-1">
                <span class="block truncate text-[11px] font-medium text-ink">
                  {{ sourceName(store.destination) }}
                </span>
                <span class="block truncate font-mono text-[9px] text-subtle">
                  {{ store.destination }}
                </span>
              </span>
              <span v-else class="text-[11px]">No destination selected</span>
              <ArrowRight class="h-3 w-3 shrink-0 text-subtle" />
            </button>

            <div class="mt-2 flex items-center gap-2 rounded border border-line/70 bg-elevated px-2.5 py-1.5">
              <input
                id="ingest-add-library"
                type="checkbox"
                class="h-3.5 w-3.5 accent-[rgb(var(--c-accent))]"
                :checked="store.addToLibrary"
                @change="store.setAddToLibrary(($event.target as HTMLInputElement).checked)"
              />
              <label for="ingest-add-library" class="text-[11px] text-ink">
                Add destination folder to library after ingest
              </label>
            </div>
          </section>
        </div>

        <section class="mt-3 rounded-lg border border-line bg-panel/60 p-3">
          <div class="mb-2 flex flex-wrap items-end justify-between gap-2">
            <div class="flex flex-wrap items-end gap-2">
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Load saved template
                </span>
                <select
                  v-model="templatePicker"
                  class="h-7 w-44 rounded border border-line bg-panel px-1.5 text-[11px] text-ink outline-none focus:border-accent/70"
                  :disabled="!store.draft.templates.length"
                  @change="chooseTemplate"
                >
                  <option value="">
                    {{ store.draft.templates.length ? 'Choose a template…' : 'No saved templates' }}
                  </option>
                  <option
                    v-for="template in store.draft.templates"
                    :key="template.id"
                    :value="template.id"
                  >
                    {{ template.name }}
                  </option>
                </select>
              </label>
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Template name
                </span>
                <div class="flex gap-1">
                  <input
                    v-model="store.templateName"
                    class="h-7 w-40 rounded border border-line bg-panel px-1.5 text-[11px] text-ink outline-none focus:border-accent/70"
                    placeholder="Client or shoot"
                    @keydown.enter.prevent="saveAsTemplate"
                  />
                  <button class="btn h-7 px-2 text-[10px]" title="Save as new template" @click="saveAsTemplate">
                    <Save class="h-3 w-3" />
                    Save
                  </button>
                  <button
                    class="btn h-7 px-2 text-[10px]"
                    :disabled="!store.selectedTemplateId"
                    title="Update the selected template"
                    @click="updateTemplate"
                  >
                    <RefreshCw class="h-3 w-3" />
                  </button>
                  <button
                    class="btn-icon-sm text-danger/70 hover:text-danger"
                    :disabled="!store.selectedTemplateId"
                    title="Delete the selected template"
                    @click="deleteSelectedTemplate"
                  >
                    <Trash2 class="h-3 w-3" />
                  </button>
                </div>
              </label>
            </div>
            <p
              class="text-[10px] text-subtle"
              title="~/Library/Application Support/com.lumio.desktop/settings.json"
            >
              Saved in Lumio settings (settings.json), not in the destination.
            </p>
          </div>

          <div class="grid gap-3 lg:grid-cols-2">
            <div class="min-w-0">
              <h3 class="mb-1 text-xs font-semibold text-ink">Destination folder</h3>
              <IngestTemplateEditor
                ref="folderEditor"
                v-model="store.draft.folderTemplate"
                kind="folder"
                aria-label="Destination folder template"
                :job-code="store.draft.jobCode"
                :custom-text="store.draft.customText"
                placeholder="{YYYY}/{MM}"
                @focus="activeTemplate = 'folder'"
              />
            </div>
            <div class="min-w-0">
              <h3 class="mb-1 text-xs font-semibold text-ink">Filename template</h3>
              <IngestTemplateEditor
                ref="filenameEditor"
                v-model="store.draft.filenameTemplate"
                kind="filename"
                aria-label="Filename template"
                :job-code="store.draft.jobCode"
                :custom-text="store.draft.customText"
                placeholder="{YYYY}{MM}{DD}_{default_filename}"
                @focus="activeTemplate = 'filename'"
              />
            </div>
          </div>

          <div class="mt-2 border-t border-line/70 pt-2">
            <IngestTokenPalette
              :disabled-tokens="blockedTemplateTokens"
              @insert="insertTemplateToken"
            />
          </div>

          <div
            class="mt-2 flex min-w-0 items-center gap-2 rounded border border-line/70 bg-elevated px-2 py-1.5"
          >
            <Eye class="h-3 w-3 shrink-0 text-subtle" />
            <span class="shrink-0 text-[10px] font-medium text-muted">Preview</span>
            <span class="min-w-0 flex-1 truncate font-mono text-[10px] text-ink" :title="previewPath">
              {{ previewPath }}
            </span>
            <span
              class="shrink-0 rounded bg-line px-1 py-0.5 text-[8px] font-medium uppercase tracking-wide text-subtle"
            >
              {{ previewSample ? 'first photo' : 'sample' }}
            </span>
          </div>
          <p v-if="store.templateError" class="mt-1 text-[10px] text-danger">
            {{ store.templateError }}
          </p>
        </section>

        <section class="mt-3 grid gap-3 lg:grid-cols-[1fr_1fr_auto]">
          <label class="block rounded-lg border border-line bg-panel/60 p-3">
            <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
              Job code
            </span>
            <input
              v-model="store.draft.jobCode"
              class="w-full rounded-md border border-line bg-panel px-2 py-1.5 font-mono text-xs text-ink outline-none focus:border-accent/70"
              placeholder="JOB-2026-014"
              spellcheck="false"
            />
          </label>
          <label class="block rounded-lg border border-line bg-panel/60 p-3">
            <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
              Custom text
            </span>
            <input
              v-model="store.draft.customText"
              class="w-full rounded-md border border-line bg-panel px-2 py-1.5 font-mono text-xs text-ink outline-none focus:border-accent/70"
              placeholder="Palawan"
              spellcheck="false"
            />
          </label>
          <div class="rounded-lg border border-line bg-panel/60 p-3">
            <div class="mb-1 text-[10px] font-medium uppercase tracking-wide text-subtle">
              Counter
            </div>
            <div class="flex items-center gap-2">
              <label class="flex items-center gap-1 text-[11px] text-muted">
                Start
                <input
                  v-model.number="store.draft.counterStart"
                  type="number"
                  min="0"
                  class="h-7 w-20 rounded border border-line bg-panel px-1.5 font-mono text-[11px] text-ink outline-none focus:border-accent/70"
                />
              </label>
              <label class="flex items-center gap-1 text-[11px] text-muted">
                Padding
                <input
                  v-model.number="store.draft.counterPadding"
                  type="number"
                  min="0"
                  max="12"
                  class="h-7 w-14 rounded border border-line bg-panel px-1.5 font-mono text-[11px] text-ink outline-none focus:border-accent/70"
                />
              </label>
            </div>
          </div>
        </section>

        <section class="mt-3 rounded-lg border border-line bg-panel/60">
          <button
            class="flex w-full items-center justify-between gap-2 px-3 py-2 text-left"
            @click="iptcOpen = !iptcOpen"
          >
            <span class="flex items-center gap-2 text-xs font-semibold text-ink">
              <FileText class="h-3.5 w-3.5 text-subtle" />
              IPTC &amp; keywords
              <span
                v-if="store.iptcRequested"
                class="rounded-full bg-accent-soft px-1.5 py-0.5 text-[9px] font-medium text-accent"
              >
                configured
              </span>
            </span>
            <span class="text-[10px] text-subtle">
              {{ iptcOpen ? 'Hide' : 'Show' }}
            </span>
          </button>
          <div v-if="iptcOpen" class="border-t border-line px-3 py-3">
            <div class="mb-2 flex flex-wrap items-center gap-2 text-[10px] text-subtle">
              <span
                class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5"
                :class="
                  store.tools?.exiftool
                    ? 'border-success/30 bg-success/10 text-success'
                    : 'border-warning/30 bg-warning/10 text-warning'
                "
              >
                <CheckCircle2 v-if="store.tools?.exiftool" class="h-2.5 w-2.5" />
                <AlertTriangle v-else class="h-2.5 w-2.5" />
                {{
                  store.tools?.exiftool
                    ? `ExifTool ${store.tools.exiftool.version}`
                    : 'ExifTool not detected'
                }}
              </span>
              <span
                >Metadata is written to XMP sidecars for RAW photos and into standalone JPEGs.
                Unrelated XMP fields are preserved. ExifTool is configured in Lumio settings.</span
              >
            </div>
            <div class="grid gap-3 lg:grid-cols-2">
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Title
                </span>
                <input
                  v-model="store.draft.iptc.title"
                  class="w-full rounded-md border border-line bg-panel px-2 py-1.5 text-xs text-ink outline-none focus:border-accent/70"
                  placeholder="Optional title"
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Description
                </span>
                <input
                  v-model="store.draft.iptc.description"
                  class="w-full rounded-md border border-line bg-panel px-2 py-1.5 text-xs text-ink outline-none focus:border-accent/70"
                  placeholder="Optional description"
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Creator
                </span>
                <input
                  v-model="store.draft.iptc.creator"
                  class="w-full rounded-md border border-line bg-panel px-2 py-1.5 text-xs text-ink outline-none focus:border-accent/70"
                  placeholder="Photographer name"
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-[10px] font-medium uppercase tracking-wide text-subtle">
                  Copyright
                </span>
                <input
                  v-model="store.draft.iptc.copyright"
                  class="w-full rounded-md border border-line bg-panel px-2 py-1.5 text-xs text-ink outline-none focus:border-accent/70"
                  placeholder="© 2026 Name"
                />
              </label>
            </div>
            <div class="mt-3">
              <div class="mb-1 text-2xs font-medium text-ink">Keywords</div>
              <IngestKeywordEditor v-model="store.draft.iptc.keywords" />
            </div>
            <p
              v-if="store.exiftoolMissing && store.iptcRequested"
              class="mt-2 flex items-start gap-1.5 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[10px] leading-relaxed text-warning"
            >
              <AlertTriangle class="mt-px h-3 w-3 shrink-0" />
              IPTC and keyword writing requires ExifTool. Install it or set its path in Settings, or
              clear the IPTC fields to continue.
            </p>
          </div>
        </section>

        <section class="mt-3 flex h-[min(460px,42vh)] min-h-[260px] flex-col overflow-hidden rounded-lg border border-line bg-panel/60">
          <div class="flex shrink-0 flex-wrap items-center gap-1 border-b border-line px-2 py-1.5">
            <button
              v-for="entry in tabs"
              :key="entry.id"
              class="inline-flex h-7 items-center gap-1.5 rounded px-2 text-[11px] font-medium transition"
              :class="
                store.tab === entry.id
                  ? 'bg-accent-soft text-accent'
                  : 'text-muted hover:bg-panel-2 hover:text-ink'
              "
              @click="store.setTab(entry.id)"
            >
              <ScrollText v-if="entry.id === 'log'" class="h-3 w-3" />
              <ListChecks v-else-if="entry.id === 'preview'" class="h-3 w-3" />
              <AlertTriangle v-else-if="entry.id === 'conflicts'" class="h-3 w-3" />
              <FileText v-else class="h-3 w-3" />
              {{ entry.label }}
              <span
                v-if="entry.count !== undefined && entry.count > 0"
                class="rounded-full bg-line px-1.5 text-[9px] text-muted"
              >
                {{ entry.count }}
              </span>
            </button>
            <span class="ml-auto pl-2 text-[10px] text-subtle">
              <template v-if="plan">
                {{ formatCount(plan.summary.entities) }} photo(s) ·
                {{ formatCount(plan.summary.files) }} file(s) ·
                {{ formatBytes(plan.summary.bytes) }}
              </template>
              <template v-else>Dry run has not been built yet</template>
            </span>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto">
            <div v-if="store.tab === 'preview'" class="p-2">
              <div
                v-if="!plan"
                class="flex h-full min-h-[180px] flex-col items-center justify-center gap-2 text-center text-[11px] text-subtle"
              >
                <ListChecks class="h-7 w-7 opacity-40" />
                <p>Choose sources and a destination, then run Dry Run.</p>
                <p class="text-[10px]">
                  The preview uses the exact scan, metadata and conflict rules that ingest uses.
                </p>
              </div>
              <template v-else>
                <div class="mb-2 flex flex-wrap items-center gap-1.5 text-[10px]">
                  <span class="rounded border border-line bg-panel px-1.5 py-0.5 text-muted">
                    {{ formatCount(plan.summary.entities) }} photos
                  </span>
                  <span class="rounded border border-line bg-panel px-1.5 py-0.5 text-muted">
                    {{ formatCount(plan.summary.files) }} files
                  </span>
                  <span class="rounded border border-line bg-panel px-1.5 py-0.5 text-muted">
                    {{ formatBytes(plan.summary.bytes) }}
                  </span>
                  <span
                    v-if="plan.summary.conflicts"
                    class="rounded border border-warning/40 bg-warning/10 px-1.5 py-0.5 text-warning"
                  >
                    {{ plan.summary.conflicts }} conflict(s)
                  </span>
                  <span
                    v-if="plan.summary.renamed"
                    class="rounded border border-warning/30 bg-warning/10 px-1.5 py-0.5 text-warning"
                  >
                    {{ plan.summary.renamed }} auto-renamed
                  </span>
                  <span
                    v-if="plan.summary.warnings"
                    class="rounded border border-warning/30 bg-warning/10 px-1.5 py-0.5 text-warning"
                  >
                    {{ plan.summary.warnings }} warning(s)
                  </span>
                  <div class="ml-auto flex items-center gap-0.5 rounded border border-line bg-panel p-0.5">
                    <button
                      class="inline-flex h-6 items-center gap-1 rounded px-1.5 text-[10px] transition"
                      :class="
                        previewView === 'table'
                          ? 'bg-accent-soft text-accent'
                          : 'text-muted hover:text-ink'
                      "
                      title="Show the preview as a table"
                      @click="previewView = 'table'"
                    >
                      <Table2 class="h-3 w-3" />
                      Table
                    </button>
                    <button
                      class="inline-flex h-6 items-center gap-1 rounded px-1.5 text-[10px] transition"
                      :class="
                        previewView === 'tree'
                          ? 'bg-accent-soft text-accent'
                          : 'text-muted hover:text-ink'
                      "
                      title="Show the destination as a folder tree"
                      @click="previewView = 'tree'"
                    >
                      <ListTree class="h-3 w-3" />
                      Tree
                    </button>
                  </div>
                </div>

                <div v-if="previewView === 'table'" class="overflow-x-auto rounded border border-line">
                  <div
                    class="grid min-w-[660px] grid-cols-[minmax(180px,1.1fr)_minmax(110px,0.7fr)_minmax(220px,1.6fr)_110px] gap-2 border-b border-line bg-panel px-2 py-1 text-[9px] font-medium uppercase tracking-wide text-subtle"
                  >
                    <span>Photo</span>
                    <span>Capture</span>
                    <span>Destination</span>
                    <span>Status</span>
                  </div>
                  <button
                    v-for="entity in plan.entities"
                    :key="entity.id"
                    class="grid min-w-[660px] w-full grid-cols-[minmax(180px,1.1fr)_minmax(110px,0.7fr)_minmax(220px,1.6fr)_110px] items-start gap-2 border-b border-line/60 px-2 py-1.5 text-left text-[10px] transition last:border-b-0 hover:bg-panel-2"
                    :class="selectedEntityId === entity.id ? 'bg-panel-2' : ''"
                    @click="selectedEntityId = selectedEntityId === entity.id ? null : entity.id"
                  >
                    <span class="min-w-0">
                      <span class="flex min-w-0 items-center gap-1">
                        <span class="truncate font-medium text-ink">{{ entity.displayName }}</span>
                        <span
                          v-if="entity.metadataTarget"
                          class="shrink-0 rounded bg-line px-1 text-[8px] uppercase text-muted"
                          :title="`Metadata: ${entity.metadataTarget.path}`"
                        >
                          iptc
                        </span>
                      </span>
                      <span class="block truncate font-mono text-[9px] text-subtle">
                        {{ entity.primaryPath }}
                      </span>
                    </span>
                    <span class="min-w-0">
                      <span class="block text-ink">{{ entity.capture.display }}</span>
                      <span class="block text-[9px] text-subtle">
                        {{ captureSourceLabel(entity.capture.source) }}
                      </span>
                    </span>
                    <span class="min-w-0 truncate font-mono text-[9px] text-muted" :title="entity.destinationDir">
                      {{ relativeDestination(entity) }}
                    </span>
                    <span>
                      <span
                        class="inline-flex rounded border px-1 py-0.5 text-[9px] font-medium"
                        :class="statusClass(entity)"
                      >
                        {{ statusLabel(entity) }}
                      </span>
                    </span>
                  </button>
                </div>

                <div v-else class="overflow-hidden rounded border border-line">
                  <div
                    class="flex items-center gap-2 border-b border-line bg-panel px-2 py-1 text-[9px] font-medium uppercase tracking-wide text-subtle"
                  >
                    <span class="min-w-0 flex-1">Destination tree</span>
                    <span class="shrink-0 normal-case tracking-normal">
                      {{ formatCount(plan.summary.entities) }} photo(s)
                    </span>
                  </div>
                  <template v-for="row in treeRows" :key="row.kind === 'folder' ? `folder:${row.path}` : row.entity?.id">
                    <div
                      v-if="row.kind === 'folder'"
                      class="flex items-center gap-1.5 border-b border-line/60 py-1 pr-2 text-[10px] last:border-b-0"
                      :style="{ paddingLeft: `${8 + row.depth * 16}px` }"
                      :title="row.path"
                    >
                      <FolderOpen class="h-3 w-3 shrink-0 text-subtle" />
                      <span class="min-w-0 flex-1 truncate font-medium text-ink">
                        {{ row.name }}
                      </span>
                      <span class="shrink-0 text-[9px] text-subtle">
                        {{ row.photoCount }} {{ row.photoCount === 1 ? 'photo' : 'photos' }}
                      </span>
                    </div>
                    <button
                      v-else-if="row.entity"
                      class="flex w-full items-center gap-1.5 border-b border-line/60 py-1 pr-2 text-left text-[10px] transition last:border-b-0 hover:bg-panel-2"
                      :class="selectedEntityId === row.entity.id ? 'bg-panel-2' : ''"
                      :style="{ paddingLeft: `${8 + row.depth * 16}px` }"
                      :title="row.path"
                      @click="togglePreviewEntity(row)"
                    >
                      <FileImage class="h-3 w-3 shrink-0 text-subtle" />
                      <span class="min-w-0 flex-1 truncate font-mono text-[9px] text-muted">
                        {{ row.name }}
                      </span>
                      <span
                        class="shrink-0 rounded border px-1 py-0.5 text-[8px] font-medium"
                        :class="statusClass(row.entity)"
                      >
                        {{ statusLabel(row.entity) }}
                      </span>
                    </button>
                  </template>
                </div>

                <div
                  v-if="selectedEntity"
                  class="mt-2 rounded border border-line bg-elevated p-2 text-[10px]"
                >
                  <div class="mb-1.5 flex items-center justify-between gap-2">
                    <span class="font-mono text-[11px] font-medium text-ink">
                      {{ selectedEntity.displayName }}
                    </span>
                    <button class="text-subtle hover:text-ink" @click="selectedEntityId = null">
                      <X class="h-3 w-3" />
                    </button>
                  </div>
                  <dl class="grid gap-x-4 gap-y-1 sm:grid-cols-2 [&>div]:min-w-0">
                    <div>
                      <dt class="text-subtle">Source folder</dt>
                      <dd class="truncate font-mono text-[9px] text-muted">
                        {{ selectedEntity.sourceDir }}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-subtle">Destination folder</dt>
                      <dd class="truncate font-mono text-[9px] text-muted">
                        {{ selectedEntity.destinationDir }}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-subtle">Capture</dt>
                      <dd class="text-muted">
                        {{ selectedEntity.capture.display }} ({{
                          captureSourceLabel(selectedEntity.capture.source)
                        }})
                      </dd>
                    </div>
                    <div>
                      <dt class="text-subtle">Counter / name</dt>
                      <dd class="font-mono text-[9px] text-muted">
                        {{ selectedEntity.destinationStem }}
                      </dd>
                    </div>
                  </dl>
                  <div v-if="selectedEntity.renamed" class="mt-1.5 text-warning">
                    Auto-renamed: {{ selectedEntity.renameReason }}
                  </div>
                  <div v-if="selectedEntity.conflict" class="mt-1.5 text-warning">
                    {{ selectedEntity.conflict.message }}
                  </div>
                  <div
                    v-for="warning in selectedEntity.warnings"
                    :key="warning"
                    class="mt-1 flex items-start gap-1 text-warning"
                  >
                    <AlertTriangle class="mt-px h-2.5 w-2.5 shrink-0" />
                    {{ warning }}
                  </div>
                  <div class="mt-1.5">
                    <div class="mb-1 text-subtle">Associated files</div>
                    <ul class="space-y-0.5">
                      <li
                        v-for="file in selectedEntity.files"
                        :key="file.path"
                        class="flex items-center gap-2"
                      >
                        <span class="w-8 shrink-0 text-[8px] font-medium text-muted">
                          {{ roleLabel(file.role) }}
                        </span>
                        <span class="min-w-0 flex-1 truncate font-mono text-[9px] text-muted">
                          {{ file.path }}
                        </span>
                        <span class="shrink-0 text-[9px] text-subtle">
                          {{ formatBytes(file.size) }}
                        </span>
                      </li>
                    </ul>
                  </div>
                  <div
                    v-if="
                      selectedEntity.iptc.title ||
                      selectedEntity.iptc.description ||
                      selectedEntity.iptc.creator ||
                      selectedEntity.iptc.copyright ||
                      selectedEntity.iptc.keywords.length
                    "
                    class="mt-1.5"
                  >
                    <div class="mb-1 text-subtle">IPTC values</div>
                    <p class="text-[9px] leading-relaxed text-muted">
                      <span v-if="selectedEntity.iptc.title">Title: {{ selectedEntity.iptc.title }}. </span>
                      <span v-if="selectedEntity.iptc.description"
                        >Description: {{ selectedEntity.iptc.description }}. </span
                      >
                      <span v-if="selectedEntity.iptc.creator"
                        >Creator: {{ selectedEntity.iptc.creator }}. </span
                      >
                      <span v-if="selectedEntity.iptc.copyright"
                        >Copyright: {{ selectedEntity.iptc.copyright }}. </span
                      >
                      <span v-if="selectedEntity.iptc.keywords.length"
                        >Keywords: {{ selectedEntity.iptc.keywords.join(', ') }}</span
                      >
                    </p>
                  </div>
                </div>
              </template>
            </div>

            <div v-else-if="store.tab === 'conflicts'" class="p-2">
              <div
                v-if="!plan"
                class="flex min-h-[180px] items-center justify-center text-[11px] text-subtle"
              >
                Run Dry Run to detect destination conflicts.
              </div>
              <div
                v-else-if="!hasConflicts"
                class="flex min-h-[180px] flex-col items-center justify-center gap-2 text-[11px] text-subtle"
              >
                <CheckCircle2 class="h-7 w-7 text-success/60" />
                <p>No destination conflicts.</p>
              </div>
              <template v-else>
                <div class="mb-2 flex flex-wrap items-center gap-1.5">
                  <span class="mr-1 text-[10px] text-subtle">
                    {{ store.conflictEntities.length }} photo(s) need a decision. Default: Skip.
                  </span>
                  <button class="btn h-7 px-2 text-[10px]" @click="applyResolutionToAll('skip')">
                    <CircleSlash class="h-3 w-3" />
                    Skip all
                  </button>
                  <button class="btn h-7 px-2 text-[10px]" @click="applyResolutionToAll('keepBoth')">
                    <Plus class="h-3 w-3" />
                    Keep both all
                  </button>
                  <button
                    class="btn h-7 border-danger/40 px-2 text-[10px] text-danger"
                    @click="applyResolutionToAll('replace')"
                  >
                    <Trash2 class="h-3 w-3" />
                    Replace all…
                  </button>
                </div>
                <div class="space-y-2">
                  <div
                    v-for="entity in store.conflictEntities"
                    :key="entity.id"
                    class="rounded border border-line bg-elevated p-2"
                  >
                    <div class="flex flex-wrap items-start justify-between gap-2">
                      <div class="min-w-0">
                        <div class="truncate text-[11px] font-medium text-ink">
                          {{ entity.displayName }}
                        </div>
                        <div class="truncate font-mono text-[9px] text-subtle">
                          {{ entity.primaryPath }}
                        </div>
                      </div>
                      <div class="flex shrink-0 gap-1">
                        <button
                          v-for="option in [
                            { value: 'skip', label: 'Skip' },
                            { value: 'keepBoth', label: 'Keep both' },
                            { value: 'replace', label: 'Replace' },
                          ] as const"
                          :key="option.value"
                          class="h-6 rounded border px-1.5 text-[10px] transition"
                          :class="
                            entity.resolution === option.value
                              ? option.value === 'replace'
                                ? 'border-danger/50 bg-danger/10 text-danger'
                                : 'border-accent/50 bg-accent-soft text-accent'
                              : 'border-line text-muted hover:border-line-strong hover:text-ink'
                          "
                          @click="resolution(entity, option.value)"
                        >
                          {{ option.label }}
                        </button>
                      </div>
                    </div>
                    <p class="mt-1 text-[10px] text-warning">{{ entity.conflict?.message }}</p>
                    <details class="mt-1">
                      <summary class="cursor-pointer text-[9px] text-subtle">
                        {{ entity.conflict?.existing.length }} existing file(s)
                      </summary>
                      <ul class="mt-1 space-y-0.5">
                        <li
                          v-for="path in entity.conflict?.existing ?? []"
                          :key="path"
                          class="truncate font-mono text-[9px] text-muted"
                        >
                          {{ path }}
                        </li>
                      </ul>
                    </details>
                    <div v-if="entity.renamed" class="mt-1 text-[10px] text-warning">
                      New name: {{ entity.destinationStem }}
                    </div>
                  </div>
                </div>
              </template>
            </div>

            <div v-else-if="store.tab === 'log'" class="p-2">
              <div class="mb-2 flex items-center gap-1">
                <button
                  v-for="filter in [
                    { value: 'all', label: 'All' },
                    { value: 'warning', label: 'Warnings' },
                    { value: 'error', label: 'Errors' },
                  ] as const"
                  :key="filter.value"
                  class="h-6 rounded border px-1.5 text-[10px] transition"
                  :class="
                    logFilter === filter.value
                      ? 'border-accent/50 bg-accent-soft text-accent'
                      : 'border-line text-muted hover:text-ink'
                  "
                  @click="logFilter = filter.value"
                >
                  {{ filter.label }}
                </button>
                <span class="ml-auto text-[10px] text-subtle">
                  {{ store.logEntries.length }} entries
                </span>
              </div>
              <div v-if="!filteredLog.length" class="py-8 text-center text-[11px] text-subtle">
                No log entries yet.
              </div>
              <ol v-else class="space-y-0.5 font-mono text-[9px] leading-relaxed">
                <li
                  v-for="(entry, index) in filteredLog"
                  :key="`${entry.at}-${index}`"
                  class="flex gap-2 rounded px-1 py-0.5"
                  :class="
                    entry.level === 'error'
                      ? 'bg-danger/5 text-danger'
                      : entry.level === 'warning'
                        ? 'bg-warning/5 text-warning'
                        : 'text-muted'
                  "
                >
                  <span class="w-[86px] shrink-0 text-subtle">{{ entry.at }}</span>
                  <span class="w-9 shrink-0 font-semibold">
                    {{ entry.level === 'error' ? 'ERR' : entry.level === 'warning' ? 'WARN' : 'INFO' }}
                  </span>
                  <span class="min-w-0 flex-1 whitespace-pre-wrap break-words">
                    {{ entry.message }}
                    <span v-if="entry.path" class="block text-subtle">{{ entry.path }}</span>
                  </span>
                </li>
              </ol>
            </div>

            <div v-else class="flex h-full min-h-0 flex-col p-2">
              <div v-if="!plan" class="flex min-h-[180px] items-center justify-center text-[11px] text-subtle">
                Run Dry Run to generate a report.
              </div>
              <template v-else>
                <div class="mb-2 flex items-center justify-between gap-2">
                  <span class="text-[10px] text-subtle">
                    Preview and execution report. Export a text copy for your records.
                  </span>
                  <button class="btn h-7 px-2 text-[10px]" @click="store.exportReport()">
                    <Download class="h-3 w-3" />
                    Export report
                  </button>
                </div>
                <pre
                  class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-words rounded border border-line bg-panel p-2 font-mono text-[9px] leading-relaxed text-muted"
                  >{{ store.reportText }}</pre
                >
              </template>
            </div>
          </div>
        </section>

        <div
          v-if="store.libraryMessage"
          class="mt-2 flex items-center gap-1.5 rounded border border-line bg-panel px-2 py-1.5 text-[10px] text-muted"
        >
          <CheckCircle2 class="h-3 w-3 text-success" />
          {{ store.libraryMessage }}
        </div>
      </div>

      <footer class="border-t border-line bg-panel py-2.5 pl-4 pr-7">
        <div class="flex flex-wrap items-center gap-3">
          <div class="min-w-0 flex-1">
            <div v-if="store.running" class="min-w-0">
              <div class="mb-1 flex items-center justify-between gap-2 text-[10px]">
                <span class="min-w-0 truncate text-muted">
                  <Loader2 class="mr-1 inline h-3 w-3 animate-spin" />
                  {{
                    store.progress.currentEntity
                      ? `Copying ${store.progress.currentEntity}`
                      : 'Preparing ingest…'
                  }}
                </span>
                <span class="shrink-0 text-subtle">
                  {{ store.progress.doneEntities }}/{{ store.progress.totalEntities }} photos ·
                  {{ formatBytes(store.progress.doneBytes) }} /
                  {{ formatBytes(store.progress.totalBytes) }}
                </span>
              </div>
              <div class="h-1.5 overflow-hidden rounded-full bg-line">
                <div
                  class="h-full rounded-full bg-accent transition-all duration-200"
                  :style="{ width: `${store.progressPercent}%` }"
                />
              </div>
            </div>
            <div v-else class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-[10px] text-subtle">
              <template v-if="plan">
                <span>
                  {{ formatCount(plan.summary.entities) }} photo(s), {{ formatCount(plan.summary.files) }}
                  file(s), {{ formatBytes(plan.summary.bytes) }}
                </span>
                <span v-if="hasConflicts" class="text-warning">
                  · {{ plan.summary.conflicts }} conflict(s)
                </span>
                <span v-if="plan.tools.exiftool" class="text-success"> · ExifTool ready</span>
                <span v-if="store.planStale" class="flex items-center gap-1 text-warning">
                  <AlertTriangle class="h-3 w-3" />
                  Settings changed since this Dry Run — run it again.
                </span>
              </template>
              <span v-else>Dry Run builds the exact ingest plan first.</span>
            </div>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <button class="btn" :disabled="store.running" @click="store.closeDialog()">Close</button>
            <button
              v-if="store.running"
              class="btn border-danger/40 text-danger"
              :disabled="store.cancelRequested"
              @click="store.cancel()"
            >
              <XCircle class="h-3.5 w-3.5" />
              {{ store.cancelRequested ? 'Cancelling…' : 'Cancel ingest' }}
            </button>
            <template v-else>
              <button
                class="btn"
                :disabled="!store.canDryRun"
                :title="
                  !store.sources.length
                    ? 'Choose at least one source'
                    : !store.destination
                      ? 'Choose a destination folder'
                      : store.exiftoolMissing
                        ? 'IPTC writing requires ExifTool — install it or clear the IPTC fields'
                        : (store.problemMessages[0] ?? 'Build the ingest plan')
                "
                @click="store.dryRun()"
              >
                <Loader2 v-if="store.planning" class="h-3.5 w-3.5 animate-spin" />
                <MonitorUp v-else class="h-3.5 w-3.5" />
                Dry Run
              </button>
              <button
                class="btn-primary"
                :disabled="!store.canRun"
                :title="
                  !plan
                    ? 'Run Dry Run first'
                    : store.outcome
                      ? 'This plan was already executed. Run Dry Run again to ingest another copy.'
                      : store.planStale
                        ? 'Settings changed since the Dry Run. Run it again to refresh the plan.'
                        : 'Copy the planned files to the destination'
                "
                @click="store.run()"
              >
                <Play class="h-3.5 w-3.5" />
                Ingest
              </button>
            </template>
          </div>
        </div>
      </footer>
    </div>
  </AppDialog>
</template>
