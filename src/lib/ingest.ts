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
  folderTemplate: '{YYYY}/{MM}',
  filenameTemplate: '{YYYY}{MM}{DD}_{default_filename}',
  jobCode: '',
  customText: '',
  counterStart: 1,
  counterPadding: 4,
  xmpNaming: 'filenameExt',
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

export function normalizeIngestSettings(settings: Partial<IngestSettings> | null): IngestSettings {
  const merged = { ...DEFAULT_INGEST_SETTINGS, ...(settings ?? {}) };
  return {
    ...merged,
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
    iptc: {
      title: settings.iptc.title,
      description: settings.iptc.description,
      creator: settings.iptc.creator,
      copyright: settings.iptc.copyright,
      keywords: [...settings.iptc.keywords],
    },
  };
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
    default_filename: PREVIEW_SAMPLE_STEM,
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
 * live preview. It deliberately uses a sample photo and never validates paths;
 * the Dry Run remains the authoritative check.
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
): IngestDestinationPreview {
  const folder = renderPreviewTemplate(settings.folderTemplate, settings, now)
    .split('/')
    .map((segment) => segment.trim())
    .filter(Boolean)
    .join('/');
  const stem = renderPreviewTemplate(settings.filenameTemplate, settings, now).trim();
  const filename = stem ? `${stem}.${PREVIEW_SAMPLE_EXTENSION}` : '';
  return {
    folder,
    filename,
    relativePath: [folder, filename].filter(Boolean).join('/'),
  };
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
