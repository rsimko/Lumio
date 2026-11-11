import type {
  CaptureSource,
  IngestEntity,
  IngestLogLevel,
  IngestOutcome,
  IngestPlan,
  IngestRules,
  IngestSettings,
  XmpNaming,
} from '@/types';

import { basename } from './format';

export type IngestTemplateKind = 'folder' | 'filename' | 'text';

export interface IngestToken {
  token: string;
  label: string;
  description: string;
  /** Literal characters are inserted as-is instead of being wrapped in braces. */
  literal?: boolean;
}

export interface IngestTokenGroup {
  label: string;
  tokens: IngestToken[];
}

export const INGEST_TOKEN_GROUPS: IngestTokenGroup[] = [
  {
    label: 'Date & time',
    tokens: [
      { token: 'YYYY', label: 'Year', description: 'Four-digit capture year' },
      { token: 'MM', label: 'Month', description: 'Two-digit capture month' },
      { token: 'DD', label: 'Day', description: 'Two-digit capture day' },
      { token: 'hh', label: 'Hour', description: 'Two-digit 24-hour capture hour' },
      { token: 'mm', label: 'Minute', description: 'Two-digit capture minute' },
      { token: 'ss', label: 'Second', description: 'Two-digit capture second' },
    ],
  },
  {
    label: 'Values',
    tokens: [
      { token: 'job_code', label: 'Job code', description: 'Job code entered for this ingest' },
      {
        token: 'custom_text',
        label: 'Custom text',
        description: 'Custom text entered for this ingest',
      },
      {
        token: 'default_filename',
        label: 'Original name',
        description: 'Original file name without its extension',
      },
      { token: 'counter', label: 'Counter', description: 'Per-photo counter' },
    ],
  },
  {
    label: 'Separators',
    tokens: [
      { token: '-', label: 'Hyphen', description: 'Insert a literal hyphen', literal: true },
      { token: '_', label: 'Underscore', description: 'Insert a literal underscore', literal: true },
      {
        token: '/',
        label: 'Slash',
        description: 'Insert a folder separator (folder template only)',
        literal: true,
      },
    ],
  },
];

export const INGEST_TOKENS: IngestToken[] = INGEST_TOKEN_GROUPS.flatMap((group) => group.tokens);

const TOKEN_SET = new Set(
  INGEST_TOKENS.filter((token) => !token.literal).map((token) => token.token),
);

const CAMERA_TOKENS = new Set([
  'make',
  'model',
  'camera',
  'camera_make',
  'camera_model',
  'lens',
  'lens_make',
  'lens_model',
  'iso',
  'focal_length',
  'aperture',
  'shutter_speed',
]);

export const DEFAULT_INGEST_SETTINGS: IngestSettings = {
  defaultSource: null,
  folderTemplate: '{YYYY}/{MM}',
  filenameTemplate: '{YYYY}{MM}{DD}_{default_filename}',
  jobCode: '',
  customText: '',
  counterStart: 1,
  counterPadding: 4,
  xmpNaming: 'filenameExt',
  renamePostfix: '_#',
  exiftoolPath: null,
  addToLibrary: true,
  templates: [],
  iptc: {
    title: '',
    description: '',
    creator: '',
    copyright: '',
    keywords: [],
  },
};

/**
 * Rename patterns are appended to a filename stem. The `#` placeholder becomes
 * the numeric suffix; a missing placeholder is appended automatically so a
 * collision can always be resolved without repeating the same name.
 */
export function normalizeRenamePostfix(value: string | null | undefined): string {
  const cleaned = (value ?? '')
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .replace(/[/\\:]/g, '')
    .trim()
    .slice(0, 32);
  const postfix = cleaned || DEFAULT_INGEST_SETTINGS.renamePostfix;
  return postfix.includes('#') ? postfix : `${postfix}#`;
}

/** Human-readable example of the rename postfix, for settings and dialogs. */
export function renamePostfixExample(
  postfix: string,
  stem = 'IMG_0001',
  extension = '.JPG',
): string {
  return `${stem}${normalizeRenamePostfix(postfix).replaceAll('#', '2')}${extension}`;
}

export function normalizeIngestSettings(settings: Partial<IngestSettings> | null): IngestSettings {
  const merged = { ...DEFAULT_INGEST_SETTINGS, ...(settings ?? {}) };
  const defaultSource =
    typeof merged.defaultSource === 'string' && merged.defaultSource.trim()
      ? merged.defaultSource.trim()
      : null;
  return {
    ...merged,
    defaultSource,
    folderTemplate: merged.folderTemplate ?? DEFAULT_INGEST_SETTINGS.folderTemplate,
    filenameTemplate: merged.filenameTemplate ?? DEFAULT_INGEST_SETTINGS.filenameTemplate,
    jobCode: merged.jobCode ?? '',
    customText: merged.customText ?? '',
    counterStart: Number.isFinite(merged.counterStart)
      ? Math.max(0, Math.round(merged.counterStart))
      : DEFAULT_INGEST_SETTINGS.counterStart,
    counterPadding: Number.isFinite(merged.counterPadding)
      ? Math.min(12, Math.max(0, Math.round(merged.counterPadding)))
      : DEFAULT_INGEST_SETTINGS.counterPadding,
    xmpNaming: merged.xmpNaming === 'filename' ? 'filename' : 'filenameExt',
    renamePostfix: normalizeRenamePostfix(merged.renamePostfix),
    exiftoolPath: merged.exiftoolPath ?? null,
    addToLibrary: Boolean(merged.addToLibrary),
    templates: Array.isArray(merged.templates) ? merged.templates : [],
    iptc: {
      title: merged.iptc?.title ?? '',
      description: merged.iptc?.description ?? '',
      creator: merged.iptc?.creator ?? '',
      copyright: merged.iptc?.copyright ?? '',
      keywords: Array.isArray(merged.iptc?.keywords) ? merged.iptc.keywords : [],
    },
  };
}

export function ingestRulesFromSettings(settings: IngestSettings): IngestRules {
  return {
    folderTemplate: settings.folderTemplate,
    filenameTemplate: settings.filenameTemplate,
    jobCode: settings.jobCode,
    customText: settings.customText,
    counterStart: settings.counterStart,
    counterPadding: settings.counterPadding,
    xmpNaming: settings.xmpNaming,
    renamePostfix: normalizeRenamePostfix(settings.renamePostfix),
    iptc: {
      title: settings.iptc.title,
      description: settings.iptc.description,
      creator: settings.iptc.creator,
      copyright: settings.iptc.copyright,
      keywords: [...settings.iptc.keywords],
    },
  };
}

/**
 * True when the stored plan no longer matches the current settings inputs.
 * The resolved ExifTool executable is intentionally not compared: auto-detect
 * stores a concrete path in the plan while the setting stays null.
 */
export function isIngestPlanStale(
  plan: Pick<IngestPlan, 'rules' | 'requestedExiftoolPath' | 'destination' | 'sources'>,
  settings: IngestSettings,
  destination: string | null,
  sources: string[],
): boolean {
  const current = normalizeIngestSettings(settings);
  if (JSON.stringify(ingestRulesFromSettings(current)) !== JSON.stringify(plan.rules)) {
    return true;
  }
  if ((plan.requestedExiftoolPath ?? null) !== (current.exiftoolPath ?? null)) return true;
  if (plan.destination !== destination) return true;
  const plannedSources = [...plan.sources].sort();
  const currentSources = [...sources].sort();
  return JSON.stringify(plannedSources) !== JSON.stringify(currentSources);
}

export function hasIptcValues(settings: IngestSettings): boolean {
  const iptc = settings.iptc;
  return Boolean(
    iptc.title.trim() ||
      iptc.description.trim() ||
      iptc.creator.trim() ||
      iptc.copyright.trim() ||
      iptc.keywords.some((keyword) => keyword.trim()),
  );
}

export interface TemplateProblem {
  kind: IngestTemplateKind;
  message: string;
}

function tokenize(template: string): { literal: string; token: string | null }[] {
  const pieces: { literal: string; token: string | null }[] = [];
  let rest = template;
  while (rest.length > 0) {
    const open = rest.indexOf('{');
    if (open === -1) {
      pieces.push({ literal: rest, token: null });
      break;
    }
    if (open > 0) pieces.push({ literal: rest.slice(0, open), token: null });
    const close = rest.indexOf('}', open + 1);
    if (close === -1) {
      pieces.push({ literal: rest.slice(open), token: null });
      break;
    }
    pieces.push({ literal: '', token: rest.slice(open + 1, close) });
    rest = rest.slice(close + 1);
  }
  return pieces;
}

function validateLiteral(literal: string, kind: IngestTemplateKind, problems: string[]): void {
  if (kind === 'text') {
    for (const character of literal) {
      if (character.charCodeAt(0) < 32) {
        problems.push('The template cannot contain control characters.');
        break;
      }
    }
    return;
  }
  if (kind === 'filename' && literal.includes('/')) {
    problems.push("The filename template cannot contain '/'.");
  }
  if (literal.includes('\\')) {
    problems.push("The template cannot contain '\\'.");
  }
  for (const character of literal) {
    if (character.charCodeAt(0) < 32) {
      problems.push('The template cannot contain control characters.');
      break;
    }
    if (':*?"<>|'.includes(character)) {
      problems.push(`The character '${character}' is not allowed in file names.`);
    }
  }
}

/**
 * Mirrors Rust `validate_template`. Returns every problem so the editor can
 * show all mistakes at once instead of one at a time.
 */
export function validateIngestTemplate(
  template: string,
  kind: IngestTemplateKind,
  options: { jobCode?: string; customText?: string } = {},
): string[] {
  const problems: string[] = [];
  if (kind === 'text' && template.trim() === '') return problems;

  const pieces = tokenize(template);
  for (const piece of pieces) {
    if (piece.token !== null) {
      const token = piece.token;
      if (token.trim() === '') {
        problems.push("Empty token '{}' in template.");
        continue;
      }
      if (token.includes('{')) {
        problems.push("Nested '{' in template.");
        continue;
      }
      if (token === 'original') {
        problems.push(
          "'{original}' is not used in Lumio. Use '{default_filename}' for the original name.",
        );
        continue;
      }
      if (CAMERA_TOKENS.has(token.toLowerCase())) {
        problems.push(
          `'{${token}}' is a camera token. Lumio deliberately excludes camera make/model tokens.`,
        );
        continue;
      }
      if (!TOKEN_SET.has(token)) {
        problems.push(`Unknown template token '{${token}}'.`);
      }
      continue;
    }
    validateLiteral(piece.literal, kind, problems);
  }

  if (template.includes('}') && template.indexOf('}') < template.indexOf('{')) {
    problems.push("Unmatched '}' in template.");
  }
  if ((template.match(/\{/g)?.length ?? 0) !== (template.match(/\}/g)?.length ?? 0)) {
    problems.push("Unbalanced '{' or '}' in template.");
  }

  if (kind === 'folder') {
    const trimmed = template.trim();
    if (trimmed.startsWith('/')) {
      problems.push('The folder template must be relative to the destination.');
    }
    if (trimmed.split('/').some((segment) => segment.trim() === '..')) {
      problems.push("The folder template cannot contain '..'.");
    }
    if (trimmed.split('/').some((segment) => segment.trim() === '.')) {
      problems.push("The folder template cannot contain '.' segments.");
    }
  }

  if (kind === 'filename') {
    const trimmed = template.trim();
    if (trimmed === '') problems.push('Enter a filename template.');
    if (trimmed === '.' || trimmed === '..') {
      problems.push('The filename template must produce a file name.');
    }
  }

  if (options.jobCode !== undefined && template.includes('{job_code}') && !options.jobCode.trim()) {
    problems.push("This template uses '{job_code}' but Job code is empty.");
  }
  if (
    options.customText !== undefined &&
    template.includes('{custom_text}') &&
    !options.customText.trim()
  ) {
    problems.push("This template uses '{custom_text}' but Custom text is empty.");
  }

  return [...new Set(problems)];
}

export function validateIngestSettings(settings: IngestSettings): TemplateProblem[] {
  const problems: TemplateProblem[] = [];
  const check = (template: string, kind: IngestTemplateKind) => {
    for (const message of validateIngestTemplate(template, kind, {
      jobCode: settings.jobCode,
      customText: settings.customText,
    })) {
      problems.push({ kind, message });
    }
  };
  check(settings.folderTemplate, 'folder');
  check(settings.filenameTemplate, 'filename');
  return problems;
}

/** The exact text a token pill inserts into a template field. */
export function ingestTokenSnippet(token: string): string {
  const known = INGEST_TOKENS.find((entry) => entry.token === token);
  return known?.literal ? token : `{${token}}`;
}

/** Insert a token at the caret, replacing any selected text. */
export function insertIngestToken(
  value: string,
  token: string,
  selectionStart: number,
  selectionEnd: number,
): { value: string; caret: number } {
  const start = Math.max(0, Math.min(selectionStart, value.length));
  const end = Math.max(start, Math.min(selectionEnd, value.length));
  const snippet = ingestTokenSnippet(token);
  return {
    value: value.slice(0, start) + snippet + value.slice(end),
    caret: start + snippet.length,
  };
}

export const PREVIEW_SAMPLE_STEM = 'IMG_1234';
export const PREVIEW_SAMPLE_EXTENSION = 'RAF';

export interface IngestPreviewSample {
  stem: string;
  extension: string;
}

/** Split a real photo path into the stem and extension shown in the preview. */
export function previewSampleFromPath(
  path: string | null | undefined,
): IngestPreviewSample | null {
  if (!path) return null;
  const name = basename(path);
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot >= name.length - 1) return null;
  return { stem: name.slice(0, dot), extension: name.slice(dot + 1) };
}

function padPreviewNumber(value: number, padding = 2): string {
  const safePadding = Number.isFinite(padding)
    ? Math.max(0, Math.min(12, Math.round(padding)))
    : 2;
  const safeValue = Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
  return String(safeValue).padStart(safePadding, '0');
}

function renderPreviewTemplate(
  template: string,
  settings: Pick<
    IngestSettings,
    'jobCode' | 'customText' | 'counterStart' | 'counterPadding'
  >,
  now: Date,
  sampleStem: string,
): string {
  const values: Record<string, string> = {
    YYYY: String(now.getFullYear()),
    MM: padPreviewNumber(now.getMonth() + 1),
    DD: padPreviewNumber(now.getDate()),
    hh: padPreviewNumber(now.getHours()),
    mm: padPreviewNumber(now.getMinutes()),
    ss: padPreviewNumber(now.getSeconds()),
    job_code: settings.jobCode.trim(),
    custom_text: settings.customText.trim(),
    default_filename: sampleStem,
    counter: padPreviewNumber(settings.counterStart, settings.counterPadding),
  };
  return template.replace(/\{([^{}]*)\}/g, (match, token: string) => values[token] ?? match);
}

export interface IngestDestinationPreview {
  folder: string;
  filename: string;
  relativePath: string;
}

/**
 * Best-effort rendering of the current folder and filename templates for the
 * live preview. Pass a real sample to show the first available photo; without
 * one it falls back to a neutral sample. The Dry Run remains authoritative.
 */
export function previewIngestDestination(
  settings: Pick<
    IngestSettings,
    | 'folderTemplate'
    | 'filenameTemplate'
    | 'jobCode'
    | 'customText'
    | 'counterStart'
    | 'counterPadding'
  >,
  now = new Date(),
  sample: IngestPreviewSample | null = null,
): IngestDestinationPreview {
  const sampleStem = sample?.stem.trim() || PREVIEW_SAMPLE_STEM;
  const sampleExtension = (sample?.extension || PREVIEW_SAMPLE_EXTENSION).replace(/^\.+/, '');
  const folder = renderPreviewTemplate(settings.folderTemplate, settings, now, sampleStem)
    .split('/')
    .map((segment) => segment.trim())
    .filter(Boolean)
    .join('/');
  const stem = renderPreviewTemplate(settings.filenameTemplate, settings, now, sampleStem).trim();
  const filename = stem ? `${stem}${sampleExtension ? `.${sampleExtension}` : ''}` : '';
  return {
    folder,
    filename,
    relativePath: [folder, filename].filter(Boolean).join('/'),
  };
}

export interface IngestDestinationTreeFolder {
  name: string;
  /** Folder path relative to the ingest destination; empty for the root. */
  path: string;
  folders: IngestDestinationTreeFolder[];
  entities: IngestEntity[];
}

/** The full destination filename of an entity, including its extension. */
export function ingestDestinationFileName(entity: IngestEntity): string {
  const primary = entity.plannedFiles.find((file) => file.primary);
  if (primary) return basename(primary.destination);
  const sourceName = basename(entity.primaryPath);
  const dot = sourceName.lastIndexOf('.');
  return `${entity.destinationStem}${dot > 0 ? sourceName.slice(dot) : ''}`;
}

function compareIngestNames(left: string, right: string): number {
  return left.localeCompare(right, undefined, { numeric: true, sensitivity: 'base' });
}

function sortIngestDestinationTree(folder: IngestDestinationTreeFolder): void {
  folder.folders.sort((left, right) => compareIngestNames(left.name, right.name));
  folder.entities.sort((left, right) =>
    compareIngestNames(ingestDestinationFileName(left), ingestDestinationFileName(right)),
  );
  for (const child of folder.folders) sortIngestDestinationTree(child);
}

/**
 * Group planned photos into the folder hierarchy they will be copied into.
 * Folders are sorted by name and photos by destination filename.
 */
export function buildIngestDestinationTree(
  plan: Pick<IngestPlan, 'destination' | 'entities'>,
): IngestDestinationTreeFolder {
  const rootPath = plan.destination.replace(/[\\/]+$/, '');
  const normalizedRoot = rootPath.replace(/\\/g, '/');
  const root: IngestDestinationTreeFolder = {
    name: basename(rootPath) || rootPath,
    path: '',
    folders: [],
    entities: [],
  };

  for (const entity of plan.entities) {
    const normalizedDir = entity.destinationDir.replace(/\\/g, '/').replace(/\/+$/, '');
    let relative = '';
    if (normalizedDir !== normalizedRoot) {
      relative = normalizedDir.startsWith(`${normalizedRoot}/`)
        ? normalizedDir.slice(normalizedRoot.length + 1)
        : normalizedDir.replace(/^\/+/, '');
    }

    let folder = root;
    for (const part of relative.split('/').filter(Boolean)) {
      const path = folder.path ? `${folder.path}/${part}` : part;
      let child = folder.folders.find((candidate) => candidate.name === part);
      if (!child) {
        child = { name: part, path, folders: [], entities: [] };
        folder.folders.push(child);
      }
      folder = child;
    }
    folder.entities.push(entity);
  }

  sortIngestDestinationTree(root);
  return root;
}

export function xmpNamingLabel(naming: XmpNaming): string {
  return naming === 'filename' ? '{filename}.xmp' : '{filename}.{ext}.xmp';
}

export function xmpNamingExample(stem: string, extension: string, naming: XmpNaming): string {
  const normalizedExtension = extension.replace(/^\./, '').toUpperCase();
  return naming === 'filename' ? `${stem}.xmp` : `${stem}.${normalizedExtension}.xmp`;
}

const CAPTURE_SOURCE_LABELS: Record<CaptureSource, string> = {
  rawExif: 'RAW metadata',
  jpegExif: 'JPEG metadata',
  xmp: 'XMP metadata',
  fileModified: 'File modified time',
};

export function captureSourceLabel(source: CaptureSource): string {
  return CAPTURE_SOURCE_LABELS[source] ?? 'Metadata';
}

const LOG_LEVEL_LABELS: Record<IngestLogLevel, string> = {
  info: 'INFO',
  warning: 'WARN',
  error: 'ERROR',
};

export function ingestLogLevelLabel(level: IngestLogLevel): string {
  return LOG_LEVEL_LABELS[level] ?? level.toUpperCase();
}

function entityFileSummary(entity: IngestEntity): string {
  return entity.files
    .map((file) => `${file.role.toUpperCase()} ${file.path}`)
    .join('\n      ');
}

export function buildIngestReport(plan: IngestPlan, outcome?: IngestOutcome | null): string {
  const lines: string[] = [];
  lines.push('Lumio ingest report');
  lines.push('===================');
  lines.push(`Plan: ${plan.id}`);
  lines.push(`Created: ${new Date(plan.createdAt).toLocaleString()}`);
  lines.push(`Destination: ${plan.destination}`);
  lines.push(`Sources:`);
  for (const source of plan.sources) lines.push(`  - ${source}`);
  lines.push('');
  lines.push('Templates');
  lines.push('---------');
  lines.push(`Folder: ${plan.rules.folderTemplate || '(destination root)'}`);
  lines.push(`Filename: ${plan.rules.filenameTemplate}`);
  lines.push(`Job code: ${plan.rules.jobCode || '(empty)'}`);
  lines.push(`Custom text: ${plan.rules.customText || '(empty)'}`);
  lines.push(
    `Counter: start ${plan.rules.counterStart}, padding ${plan.rules.counterPadding}`,
  );
  lines.push(`XMP naming: ${xmpNamingLabel(plan.rules.xmpNaming)}`);
  lines.push(
    `Metadata reader: ${
      plan.tools.exiftool
        ? `ExifTool ${plan.tools.exiftool.version} (${plan.tools.exiftool.path})`
        : 'built-in RAW/JPEG reader'
    }`,
  );
  if (!plan.rules.iptc.title.trim() &&
    !plan.rules.iptc.description.trim() &&
    !plan.rules.iptc.creator.trim() &&
    !plan.rules.iptc.copyright.trim() &&
    !plan.rules.iptc.keywords.some((keyword) => keyword.trim())) {
    lines.push('IPTC: none');
  } else {
    lines.push('IPTC values');
    lines.push(`  Title: ${plan.rules.iptc.title || '(empty)'}`);
    lines.push(`  Description: ${plan.rules.iptc.description || '(empty)'}`);
    lines.push(`  Creator: ${plan.rules.iptc.creator || '(empty)'}`);
    lines.push(`  Copyright: ${plan.rules.iptc.copyright || '(empty)'}`);
    lines.push(
      `  Keywords: ${
        plan.rules.iptc.keywords.filter((keyword) => keyword.trim()).join(', ') || '(empty)'
      }`,
    );
  }
  lines.push('');
  lines.push('Summary');
  lines.push('-------');
  lines.push(`Photos: ${plan.summary.entities}`);
  lines.push(`Files: ${plan.summary.files}`);
  lines.push(`Bytes: ${plan.summary.bytes}`);
  lines.push(`Conflicts: ${plan.summary.conflicts}`);
  lines.push(`Auto-renamed: ${plan.summary.renamed}`);
  lines.push(`Warnings: ${plan.summary.warnings}`);
  lines.push(`Errors: ${plan.summary.errors}`);
  if (outcome) {
    lines.push('');
    lines.push('Result');
    lines.push('------');
    lines.push(`Copied photos: ${outcome.copiedEntities}`);
    lines.push(`Skipped photos: ${outcome.skippedEntities}`);
    lines.push(`Failed photos: ${outcome.failedEntities}`);
    lines.push(`Copied files: ${outcome.copiedFiles}`);
    lines.push(`Copied bytes: ${outcome.copiedBytes}`);
    lines.push(`Elapsed: ${outcome.elapsedMs} ms`);
    lines.push(`Cancelled: ${outcome.cancelled ? 'yes' : 'no'}`);
  }
  lines.push('');
  lines.push('Photos');
  lines.push('------');
  for (const [index, entity] of plan.entities.entries()) {
    lines.push(`${index + 1}. ${entity.displayName} [${entity.status}]`);
    lines.push(`   Source: ${entity.primaryPath}`);
    lines.push(
      `   Capture: ${entity.capture.display} (${captureSourceLabel(entity.capture.source)})`,
    );
    lines.push(`   Destination folder: ${entity.destinationDir}`);
    lines.push(`   Destination name: ${entity.destinationStem}`);
    lines.push(`   Files:\n      ${entityFileSummary(entity)}`);
    if (entity.renamed) {
      lines.push(`   Renamed: ${entity.renameReason ?? 'yes'}`);
    }
    if (entity.conflict) {
      lines.push(`   Conflict (${entity.resolution}): ${entity.conflict.message}`);
    }
    for (const warning of entity.warnings) lines.push(`   Warning: ${warning}`);
    if (entity.metadataTarget) {
      lines.push(
        `   Metadata target: ${entity.metadataTarget.kind} ${entity.metadataTarget.path}`,
      );
    }
    const iptc = entity.iptc;
    if (
      iptc.title ||
      iptc.description ||
      iptc.creator ||
      iptc.copyright ||
      iptc.keywords.length > 0
    ) {
      lines.push(
        `   IPTC: title=${iptc.title ?? '—'}; description=${iptc.description ?? '—'}; creator=${
          iptc.creator ?? '—'
        }; copyright=${iptc.copyright ?? '—'}; keywords=${iptc.keywords.join(', ') || '—'}`,
      );
    }
    lines.push('');
  }
  if (plan.warnings.length > 0) {
    lines.push('Plan warnings');
    lines.push('-------------');
    for (const warning of plan.warnings) {
      lines.push(`- ${warning.message}${warning.path ? ` (${warning.path})` : ''}`);
    }
    lines.push('');
  }
  lines.push('Operation log');
  lines.push('-------------');
  for (const entry of plan.log) {
    lines.push(
      `[${entry.at}] ${ingestLogLevelLabel(entry.level)} ${entry.message}${
        entry.path ? ` (${entry.path})` : ''
      }`,
    );
  }
  if (outcome) {
    lines.push('');
    lines.push('Final log');
    lines.push('---------');
    for (const entry of outcome.log) {
      lines.push(
        `[${entry.at}] ${ingestLogLevelLabel(entry.level)} ${entry.message}${
          entry.path ? ` (${entry.path})` : ''
        }`,
      );
    }
  }
  return lines.join('\n');
}

export function defaultReportName(plan: IngestPlan): string {
  const stamp = new Date(plan.createdAt)
    .toISOString()
    .replace(/[:.]/g, '-')
    .slice(0, 19);
  return `lumio-ingest-${stamp}.txt`;
}
