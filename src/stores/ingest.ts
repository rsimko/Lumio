import { open as openPathDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { api, errorMessage } from '@/api';
import {
  DEFAULT_INGEST_SETTINGS,
  buildIngestReport,
  defaultReportName,
  hasIptcValues,
  ingestRulesFromSettings,
  isIngestPlanStale,
  normalizeIngestSettings,
  validateIngestSettings,
} from '@/lib/ingest';
import { useLibraryStore } from '@/stores/library';
import type {
  IngestConflictResolution,
  IngestEntity,
  IngestEvent,
  IngestLogEntry,
  IngestOutcome,
  IngestPlan,
  IngestSettings,
  IngestTemplate,
  IngestTools,
} from '@/types';

export type IngestTab = 'preview' | 'conflicts' | 'log' | 'report';

export interface IngestProgress {
  totalEntities: number;
  totalFiles: number;
  totalBytes: number;
  doneEntities: number;
  doneFiles: number;
  doneBytes: number;
  skipped: number;
  failed: number;
  currentEntity: string | null;
  currentIndex: number;
  currentFile: number;
  currentFileTotal: number;
}

const PHOTO_EXTENSIONS = [
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
];

function emptyProgress(): IngestProgress {
  return {
    totalEntities: 0,
    totalFiles: 0,
    totalBytes: 0,
    doneEntities: 0,
    doneFiles: 0,
    doneBytes: 0,
    skipped: 0,
    failed: 0,
    currentEntity: null,
    currentIndex: 0,
    currentFile: 0,
    currentFileTotal: 0,
  };
}

function cloneSettings(settings: IngestSettings): IngestSettings {
  return {
    ...settings,
    templates: settings.templates.map((template) => ({ ...template })),
    iptc: { ...settings.iptc, keywords: [...settings.iptc.keywords] },
  };
}

function uniquePaths(paths: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const path of paths) {
    if (!path || seen.has(path)) continue;
    seen.add(path);
    result.push(path);
  }
  return result;
}

function newTemplateId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `template-${Date.now()}-${Math.random()}`;
}

export const useIngestStore = defineStore('ingest', () => {
  const library = useLibraryStore();

  const open = ref(false);
  const tab = ref<IngestTab>('preview');
  const sources = ref<string[]>([]);
  const destination = ref<string | null>(null);
  const draft = ref<IngestSettings>(cloneSettings(DEFAULT_INGEST_SETTINGS));
  const addToLibrary = ref(true);

  const planning = ref(false);
  const planError = ref<string | null>(null);
  const plan = ref<IngestPlan | null>(null);

  const detectingTools = ref(false);
  const tools = ref<IngestTools | null>(null);

  const running = ref(false);
  const cancelRequested = ref(false);
  const runError = ref<string | null>(null);
  const outcome = ref<IngestOutcome | null>(null);
  const progress = ref<IngestProgress>(emptyProgress());
  const liveLog = ref<IngestLogEntry[]>([]);
  const libraryMessage = ref<string | null>(null);

  const templateName = ref('');
  const templateError = ref<string | null>(null);
  const selectedTemplateId = ref<string | null>(null);

  const problems = computed(() => validateIngestSettings(draft.value));
  const problemMessages = computed(() => problems.value.map((problem) => problem.message));
  const iptcRequested = computed(() => hasIptcValues(draft.value));
  const exiftoolMissing = computed(() => iptcRequested.value && !tools.value?.exiftool);
  const canDryRun = computed(
    () =>
      !planning.value &&
      !running.value &&
      sources.value.length > 0 &&
      Boolean(destination.value) &&
      problems.value.length === 0 &&
      !exiftoolMissing.value,
  );
  /**
   * The backend executes the stored plan exactly as it was built. If any
   * plan-affecting value changed since the Dry Run, running would silently
   * ignore the edit, so the plan is marked stale instead.
   */
  const planStale = computed(() => {
    if (!plan.value) return false;
    return isIngestPlanStale(plan.value, draft.value, destination.value, sources.value);
  });

  const canRun = computed(
    () =>
      !planning.value &&
      !running.value &&
      !outcome.value &&
      !planStale.value &&
      Boolean(plan.value) &&
      (plan.value?.entities.length ?? 0) > 0,
  );
  const conflictEntities = computed(
    () => plan.value?.entities.filter((entity) => entity.conflict) ?? [],
  );
  const logEntries = computed<IngestLogEntry[]>(() => {
    if (outcome.value) return outcome.value.log;
    return [...(plan.value?.log ?? []), ...liveLog.value];
  });
  const reportText = computed(() =>
    plan.value ? buildIngestReport(plan.value, outcome.value) : '',
  );
  const copiedDestination = computed(() => outcome.value?.destination ?? plan.value?.destination ?? null);
  const progressPercent = computed(() => {
    // Prefer bytes: a single large RAW can take a while, and this keeps the
    // bar moving as files are staged instead of only when a photo completes.
    if (progress.value.totalBytes > 0) {
      const ratio = progress.value.doneBytes / progress.value.totalBytes;
      return Math.min(100, Math.round(ratio * 100));
    }
    if (progress.value.totalEntities > 0) {
      return Math.min(
        100,
        Math.round((progress.value.doneEntities / progress.value.totalEntities) * 100),
      );
    }
    return 0;
  });

  function resetPlan(): void {
    plan.value = null;
    outcome.value = null;
    runError.value = null;
    planError.value = null;
    liveLog.value = [];
    progress.value = emptyProgress();
    cancelRequested.value = false;
    libraryMessage.value = null;
    tab.value = 'preview';
  }

  async function detectTools(): Promise<void> {
    detectingTools.value = true;
    try {
      tools.value = await api.detectIngestTools(draft.value.exiftoolPath);
    } catch (error) {
      planError.value = errorMessage(error);
    } finally {
      detectingTools.value = false;
    }
  }

  async function openDialog(): Promise<void> {
    if (open.value) return;
    resetPlan();
    draft.value = cloneSettings(library.ingestSettings);
    addToLibrary.value = draft.value.addToLibrary;
    sources.value = [];
    destination.value = null;
    templateName.value = '';
    templateError.value = null;
    selectedTemplateId.value = null;
    open.value = true;
    await detectTools();
  }

  function closeDialog(): void {
    if (running.value) return;
    open.value = false;
  }

  /** Clear the completed run and begin a fresh ingest with the same settings. */
  function startNewIngest(): void {
    if (running.value) return;
    resetPlan();
    sources.value = [];
    destination.value = null;
    templateName.value = '';
    templateError.value = null;
    selectedTemplateId.value = null;
    addToLibrary.value = draft.value.addToLibrary;
  }

  function setTab(next: IngestTab): void {
    tab.value = next;
  }

  function setSources(paths: string[]): void {
    sources.value = uniquePaths(paths);
    if (plan.value) resetPlan();
  }

  async function addSourceFolders(): Promise<void> {
    const picked = await openPathDialog({
      directory: true,
      multiple: true,
      title: 'Choose ingest source folders',
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    setSources([...sources.value, ...paths]);
  }

  async function addSourceFiles(): Promise<void> {
    const picked = await openPathDialog({
      directory: false,
      multiple: true,
      title: 'Choose ingest source files',
      filters: [{ name: 'Photos and XMP', extensions: PHOTO_EXTENSIONS }],
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    setSources([...sources.value, ...paths]);
  }

  function removeSource(path: string): void {
    setSources(sources.value.filter((source) => source !== path));
  }

  function clearSources(): void {
    setSources([]);
  }

  async function chooseDestination(): Promise<void> {
    const picked = await openPathDialog({
      directory: true,
      multiple: false,
      title: 'Choose the ingest destination',
    });
    if (!picked || Array.isArray(picked)) return;
    destination.value = picked;
    if (plan.value) resetPlan();
  }

  function setDestination(path: string | null): void {
    destination.value = path;
    if (plan.value) resetPlan();
  }

  async function persistDraft(): Promise<void> {
    try {
      draft.value = await library.saveIngestSettings(draft.value);
    } catch (error) {
      library.addToast(errorMessage(error), 'error');
    }
  }

  async function dryRun(): Promise<void> {
    if (!canDryRun.value || !destination.value) return;
    planning.value = true;
    planError.value = null;
    outcome.value = null;
    liveLog.value = [];
    libraryMessage.value = null;
    try {
      await persistDraft();
      const normalized = normalizeIngestSettings(draft.value);
      const nextPlan = await api.createIngestPlan({
        sources: sources.value,
        destination: destination.value,
        rules: ingestRulesFromSettings(normalized),
        exiftoolPath: normalized.exiftoolPath,
      });
      plan.value = nextPlan;
      // The backend canonicalizes the destination. Adopt that exact path so the
      // stale-plan check compares like with like.
      destination.value = nextPlan.destination;
      progress.value = {
        ...emptyProgress(),
        totalEntities: nextPlan.summary.entities,
        totalFiles: nextPlan.summary.files,
        totalBytes: nextPlan.summary.bytes,
      };
      tab.value = nextPlan.summary.conflicts > 0 ? 'conflicts' : 'preview';
    } catch (error) {
      planError.value = errorMessage(error);
    } finally {
      planning.value = false;
    }
  }

  async function setResolution(
    entityId: string,
    resolution: IngestConflictResolution,
  ): Promise<void> {
    if (!plan.value || running.value || outcome.value) return;
    try {
      plan.value = await api.setIngestResolution(plan.value.id, entityId, resolution);
    } catch (error) {
      planError.value = errorMessage(error);
    }
  }

  function handleEvent(event: IngestEvent): void {
    switch (event.type) {
      case 'started':
        progress.value = {
          ...emptyProgress(),
          totalEntities: event.totalEntities,
          totalFiles: event.totalFiles,
          totalBytes: event.totalBytes,
        };
        break;
      case 'entityStarted':
        progress.value = {
          ...progress.value,
          currentEntity: event.displayName,
          currentIndex: event.index,
          currentFile: 0,
          currentFileTotal: 0,
        };
        break;
      case 'fileCopied':
        progress.value = {
          ...progress.value,
          doneFiles: progress.value.doneFiles + 1,
          doneBytes: progress.value.doneBytes + event.bytes,
          currentFile: event.index,
          currentFileTotal: event.total,
        };
        break;
      case 'entityCompleted':
        progress.value = {
          ...progress.value,
          doneEntities: progress.value.doneEntities + 1,
          currentEntity: null,
        };
        break;
      case 'entitySkipped':
        progress.value = {
          ...progress.value,
          doneEntities: progress.value.doneEntities + 1,
          skipped: progress.value.skipped + 1,
          currentEntity: null,
        };
        liveLog.value.push({
          at: new Date().toLocaleTimeString(),
          level: 'warning',
          message: `Skipped ${event.displayName}: ${event.reason}`,
          path: null,
          entityId: event.entityId,
        });
        break;
      case 'entityFailed':
        progress.value = {
          ...progress.value,
          doneEntities: progress.value.doneEntities + 1,
          failed: progress.value.failed + 1,
          currentEntity: null,
        };
        liveLog.value.push({
          at: new Date().toLocaleTimeString(),
          level: 'error',
          message: `Failed ${event.displayName}: ${event.message}`,
          path: null,
          entityId: event.entityId,
        });
        break;
      case 'warning':
        liveLog.value.push({
          at: new Date().toLocaleTimeString(),
          level: 'warning',
          message: event.message,
          path: null,
          entityId: event.entityId,
        });
        break;
      case 'log':
        liveLog.value.push(event.entry);
        break;
    }
  }

  async function addDestinationToLibrary(): Promise<void> {
    const target = copiedDestination.value;
    if (!target) return;
    try {
      const result = await library.registerRoot(target, { select: false, toast: false });
      libraryMessage.value = result.alreadyInLibrary
        ? `${result.root.name} is already in the library.`
        : `Added ${result.root.name} to the library.`;
    } catch (error) {
      libraryMessage.value = `Could not add the destination to the library: ${errorMessage(
        error,
      )}`;
    }
  }

  async function run(): Promise<void> {
    if (!plan.value || !canRun.value) return;
    running.value = true;
    runError.value = null;
    outcome.value = null;
    libraryMessage.value = null;
    liveLog.value = [];
    cancelRequested.value = false;
    progress.value = {
      ...emptyProgress(),
      totalEntities: plan.value.summary.entities,
      totalFiles: plan.value.summary.files,
      totalBytes: plan.value.summary.bytes,
    };
    tab.value = 'log';
    try {
      const result = await api.runIngest(plan.value.id, handleEvent);
      outcome.value = result;
      progress.value = {
        ...progress.value,
        doneEntities: result.copiedEntities + result.skippedEntities + result.failedEntities,
        doneFiles: result.copiedFiles,
        doneBytes: result.copiedBytes,
        skipped: result.skippedEntities,
        failed: result.failedEntities,
        currentEntity: null,
      };
      tab.value = 'report';
      if (addToLibrary.value) await addDestinationToLibrary();
      library.addToast(
        result.cancelled
          ? `Ingest cancelled. ${result.copiedEntities} photo(s) copied.`
          : `Ingest finished. ${result.copiedEntities} photo(s) copied, ${result.failedEntities} failed.`,
        result.failedEntities > 0 ? 'error' : 'success',
        6000,
      );
      if (result.copiedEntities > 0) {
        await library.refresh();
        await library.refreshHiddenAssets();
      }
    } catch (error) {
      runError.value = errorMessage(error);
      tab.value = 'log';
    } finally {
      running.value = false;
    }
  }

  async function cancel(): Promise<void> {
    if (!plan.value || !running.value) return;
    cancelRequested.value = true;
    try {
      await api.cancelIngest(plan.value.id);
    } catch (error) {
      runError.value = errorMessage(error);
    }
  }

  async function exportReport(): Promise<void> {
    if (!plan.value) return;
    const directory = plan.value.destination.replace(/\/+$/, '');
    const picked = await saveDialog({
      title: 'Export ingest report',
      defaultPath: `${directory}/${defaultReportName(plan.value)}`,
      filters: [{ name: 'Text report', extensions: ['txt'] }],
    });
    if (!picked) return;
    try {
      await api.writeTextFile(picked, reportText.value);
      library.addToast('Ingest report exported.', 'success');
    } catch (error) {
      library.addToast(errorMessage(error), 'error');
    }
  }

  function applyTemplate(id: string): void {
    const template = draft.value.templates.find((candidate) => candidate.id === id);
    if (!template) return;
    draft.value.folderTemplate = template.folderTemplate;
    draft.value.filenameTemplate = template.filenameTemplate;
    selectedTemplateId.value = template.id;
    templateName.value = template.name;
    templateError.value = null;
    if (plan.value) resetPlan();
  }

  async function saveTemplate(name: string, id?: string): Promise<void> {
    const trimmed = name.trim();
    if (!trimmed) {
      templateError.value = 'Enter a template name.';
      return;
    }
    const now = Date.now();
    const existing = id
      ? draft.value.templates.find((template) => template.id === id)
      : draft.value.templates.find(
          (template) => template.name.toLowerCase() === trimmed.toLowerCase(),
        );
    const next: IngestTemplate = existing
      ? {
          ...existing,
          name: trimmed,
          folderTemplate: draft.value.folderTemplate,
          filenameTemplate: draft.value.filenameTemplate,
          updatedAt: now,
        }
      : {
          id: newTemplateId(),
          name: trimmed,
          folderTemplate: draft.value.folderTemplate,
          filenameTemplate: draft.value.filenameTemplate,
          createdAt: now,
          updatedAt: now,
        };
    draft.value.templates = existing
      ? draft.value.templates.map((template) =>
          template.id === existing.id ? next : template,
        )
      : [...draft.value.templates, next];
    templateName.value = next.name;
    selectedTemplateId.value = next.id;
    templateError.value = null;
    await persistDraft();
  }

  async function deleteTemplate(id: string): Promise<void> {
    draft.value.templates = draft.value.templates.filter((template) => template.id !== id);
    if (selectedTemplateId.value === id) {
      selectedTemplateId.value = null;
      templateName.value = '';
    }
    await persistDraft();
  }

  async function setAddToLibrary(value: boolean): Promise<void> {
    addToLibrary.value = value;
    draft.value.addToLibrary = value;
    await persistDraft();
  }

  function entityFor(id: string): IngestEntity | undefined {
    return plan.value?.entities.find((entity) => entity.id === id);
  }

  return {
    open,
    tab,
    sources,
    destination,
    draft,
    addToLibrary,
    planning,
    planError,
    plan,
    detectingTools,
    tools,
    running,
    cancelRequested,
    runError,
    outcome,
    progress,
    progressPercent,
    liveLog,
    logEntries,
    libraryMessage,
    templateName,
    templateError,
    selectedTemplateId,
    problems,
    problemMessages,
    iptcRequested,
    exiftoolMissing,
    canDryRun,
    canRun,
    planStale,
    conflictEntities,
    reportText,
    copiedDestination,
    openDialog,
    closeDialog,
    startNewIngest,
    setTab,
    setSources,
    addSourceFolders,
    addSourceFiles,
    removeSource,
    clearSources,
    chooseDestination,
    setDestination,
    detectTools,
    persistDraft,
    dryRun,
    setResolution,
    run,
    cancel,
    exportReport,
    applyTemplate,
    saveTemplate,
    deleteTemplate,
    setAddToLibrary,
    resetPlan,
    entityFor,
  };
});
