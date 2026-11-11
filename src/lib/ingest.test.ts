import { describe, expect, it } from 'vitest';

import {
  DEFAULT_INGEST_SETTINGS,
  INGEST_TOKEN_GROUPS,
  buildIngestDestinationTree,
  buildIngestReport,
  captureSourceLabel,
  defaultReportName,
  hasIptcValues,
  ingestDestinationFileName,
  ingestRulesFromSettings,
  ingestTokenSnippet,
  insertIngestToken,
  isIngestPlanStale,
  normalizeIngestSettings,
  previewIngestDestination,
  previewSampleFromPath,
  validateIngestSettings,
  validateIngestTemplate,
  xmpNamingExample,
  xmpNamingLabel,
} from '@/lib/ingest';
import type { IngestEntity, IngestPlan } from '@/types';

describe('ingest settings', () => {
  it('has safe defaults that match the backend', () => {
    expect(DEFAULT_INGEST_SETTINGS.folderTemplate).toBe('{YYYY}/{MM}');
    expect(DEFAULT_INGEST_SETTINGS.filenameTemplate).toBe('{YYYY}{MM}{DD}_{default_filename}');
    expect(DEFAULT_INGEST_SETTINGS.counterStart).toBe(1);
    expect(DEFAULT_INGEST_SETTINGS.counterPadding).toBe(4);
    expect(DEFAULT_INGEST_SETTINGS.xmpNaming).toBe('filenameExt');
    expect(DEFAULT_INGEST_SETTINGS.addToLibrary).toBe(true);
  });

  it('normalizes missing and invalid persisted values', () => {
    const settings = normalizeIngestSettings({
      filenameTemplate: '{counter}_{default_filename}',
      counterStart: -4,
      counterPadding: 42,
      xmpNaming: 'filename',
      iptc: { title: 'Title', description: '', creator: '', copyright: '', keywords: ['a'] },
    });
    expect(settings.filenameTemplate).toBe('{counter}_{default_filename}');
    expect(settings.counterStart).toBe(0);
    expect(settings.counterPadding).toBe(12);
    expect(settings.xmpNaming).toBe('filename');
    expect(settings.iptc.keywords).toEqual(['a']);
    expect(settings.addToLibrary).toBe(true);
  });

  it('builds request rules without sharing arrays', () => {
    const settings = normalizeIngestSettings({
      iptc: { title: 'T', description: '', creator: '', copyright: '', keywords: ['one'] },
    });
    const rules = ingestRulesFromSettings(settings);
    rules.iptc.keywords.push('two');
    expect(settings.iptc.keywords).toEqual(['one']);
    expect(rules.folderTemplate).toBe(settings.folderTemplate);
  });

  it('detects whether IPTC values were supplied', () => {
    expect(hasIptcValues(normalizeIngestSettings(null))).toBe(false);
    expect(
      hasIptcValues(
        normalizeIngestSettings({
          iptc: { title: '', description: '', creator: '', copyright: '', keywords: [' travel '] },
        }),
      ),
    ).toBe(true);
  });

  it('labels XMP naming options and examples', () => {
    expect(xmpNamingLabel('filenameExt')).toBe('{filename}.{ext}.xmp');
    expect(xmpNamingLabel('filename')).toBe('{filename}.xmp');
    expect(xmpNamingExample('NEW', 'cr3', 'filenameExt')).toBe('NEW.CR3.xmp');
    expect(xmpNamingExample('NEW', '.CR3', 'filename')).toBe('NEW.xmp');
  });
});

describe('ingest templates', () => {
  it('accepts valid date, value and literal tokens', () => {
    expect(
      validateIngestTemplate(
        '{YYYY}-{MM}-{DD}_{hh}{mm}{ss}_{job_code}_{custom_text}_{default_filename}_{counter}',
        'filename',
      ),
    ).toEqual([]);
    expect(validateIngestTemplate('{YYYY}/{MM}/Job_{job_code}', 'folder')).toEqual([]);
  });

  it('rejects unknown, camera and legacy tokens', () => {
    expect(validateIngestTemplate('{original}', 'filename')).toEqual([
      "'{original}' is not used in Lumio. Use '{default_filename}' for the original name.",
    ]);
    expect(validateIngestTemplate('{make}_{model}', 'filename')).toEqual([
      "'{make}' is a camera token. Lumio deliberately excludes camera make/model tokens.",
      "'{model}' is a camera token. Lumio deliberately excludes camera make/model tokens.",
    ]);
    expect(validateIngestTemplate('{nope}', 'filename')).toEqual([
      "Unknown template token '{nope}'.",
    ]);
  });

  it('rejects separators and filesystem characters in filenames', () => {
    expect(validateIngestTemplate('a/b', 'filename')).toContain(
      "The filename template cannot contain '/'.",
    );
    expect(validateIngestTemplate('a:b', 'filename')).toContain(
      "The character ':' is not allowed in file names.",
    );
    expect(validateIngestTemplate('a\\b', 'filename')).toContain(
      "The template cannot contain '\\'.",
    );
  });

  it('validates folder templates as relative paths', () => {
    expect(validateIngestTemplate('/absolute', 'folder')).toContain(
      'The folder template must be relative to the destination.',
    );
    expect(validateIngestTemplate('../outside', 'folder')).toContain(
      "The folder template cannot contain '..'.",
    );
    expect(validateIngestTemplate('./inside', 'folder')).toContain(
      "The folder template cannot contain '.' segments.",
    );
  });

  it('reports missing job code and custom text inputs', () => {
    const problems = validateIngestTemplate('{job_code}_{custom_text}', 'filename', {
      jobCode: ' ',
      customText: '',
    });
    expect(problems).toContain("This template uses '{job_code}' but Job code is empty.");
    expect(problems).toContain("This template uses '{custom_text}' but Custom text is empty.");
  });

  it('allows text templates to be empty and ignores path rules', () => {
    expect(validateIngestTemplate('', 'text')).toEqual([]);
    expect(validateIngestTemplate('Fjord / Morning', 'text')).toEqual([]);
    expect(validateIngestTemplate('Morning: blue hour?', 'text')).toEqual([]);
  });

  it('validates every template in a settings object', () => {
    const problems = validateIngestSettings(
      normalizeIngestSettings({
        folderTemplate: '/bad',
        filenameTemplate: '{original}',
        iptc: {
          title: '{unknown}',
          description: '',
          creator: '',
          copyright: '',
          keywords: ['{counter}'],
        },
      }),
    );
    expect(problems.map((problem) => problem.kind)).toContain('folder');
    expect(problems.map((problem) => problem.kind)).toContain('filename');
    // IPTC values are literal text and are never parsed as templates.
    expect(problems.map((problem) => problem.kind)).not.toContain('text');
  });
});

describe('token insertion', () => {
  it('exposes separator tokens as literal snippets', () => {
    const separators = INGEST_TOKEN_GROUPS.find((group) => group.label === 'Separators');
    expect(separators?.tokens.map((token) => token.token)).toEqual(['-', '_', '/']);
    expect(ingestTokenSnippet('YYYY')).toBe('{YYYY}');
    expect(ingestTokenSnippet('-')).toBe('-');
    expect(ingestTokenSnippet('/')).toBe('/');
  });

  it('inserts at the caret and moves the caret after the token', () => {
    expect(insertIngestToken('AB', 'YYYY', 1, 1)).toEqual({
      value: 'A{YYYY}B',
      caret: 7,
    });
  });

  it('inserts separators without braces', () => {
    expect(insertIngestToken('AB', '-', 1, 1)).toEqual({ value: 'A-B', caret: 2 });
    expect(insertIngestToken('AB', '/', 1, 1)).toEqual({ value: 'A/B', caret: 2 });
  });

  it('replaces selected text', () => {
    expect(insertIngestToken('ABCD', 'counter', 1, 3)).toEqual({
      value: 'A{counter}D',
      caret: 10,
    });
  });

  it('clamps invalid selections', () => {
    expect(insertIngestToken('AB', 'MM', 99, 99)).toEqual({ value: 'AB{MM}', caret: 6 });
    expect(insertIngestToken('AB', 'MM', -2, 0)).toEqual({ value: '{MM}AB', caret: 4 });
  });
});

describe('stale plan detection', () => {
  const settings = normalizeIngestSettings({
    folderTemplate: '{YYYY}/{MM}',
    filenameTemplate: '{default_filename}',
  });
  const plan = {
    rules: ingestRulesFromSettings(settings),
    exiftoolPath: '/opt/homebrew/bin/exiftool',
    requestedExiftoolPath: null,
    destination: '/destination',
    sources: ['/source/a', '/source/b'],
  };

  it('stays fresh when ExifTool was auto-detected and the setting is still null', () => {
    expect(isIngestPlanStale(plan, settings, '/destination', ['/source/b', '/source/a'])).toBe(
      false,
    );
  });

  it('becomes stale when the ExifTool setting changes', () => {
    expect(
      isIngestPlanStale(
        plan,
        { ...settings, exiftoolPath: '/custom/exiftool' },
        '/destination',
        plan.sources,
      ),
    ).toBe(true);
  });

  it('becomes stale when templates, destination or sources change', () => {
    expect(
      isIngestPlanStale(
        plan,
        { ...settings, filenameTemplate: '{counter}_{default_filename}' },
        '/destination',
        plan.sources,
      ),
    ).toBe(true);
    expect(isIngestPlanStale(plan, settings, '/other', plan.sources)).toBe(true);
    expect(isIngestPlanStale(plan, settings, '/destination', ['/source/a'])).toBe(true);
  });
});

describe('destination preview', () => {
  it('renders the folder path, filename and sample extension', () => {
    const preview = previewIngestDestination(
      normalizeIngestSettings({
        folderTemplate: '{YYYY}/{MM}',
        filenameTemplate: '{YYYY}{MM}{DD}_{default_filename}_{counter}',
        counterStart: 7,
        counterPadding: 3,
      }),
      new Date(2026, 10, 11, 14, 5, 9),
    );
    expect(preview).toEqual({
      folder: '2026/11',
      filename: '20261111_IMG_1234_007.RAF',
      relativePath: '2026/11/20261111_IMG_1234_007.RAF',
    });
  });

  it('uses job code and custom text and leaves unknown tokens alone', () => {
    const preview = previewIngestDestination(
      normalizeIngestSettings({
        folderTemplate: 'Job_{job_code}/{custom_text}',
        filenameTemplate: '{job_code}_{nope}_{default_filename}',
        jobCode: 'JOB-1',
        customText: 'Palawan',
      }),
      new Date(2026, 0, 2, 3, 4, 5),
    );
    expect(preview.folder).toBe('Job_JOB-1/Palawan');
    expect(preview.filename).toBe('JOB-1_{nope}_IMG_1234.RAF');
    expect(preview.relativePath).toBe('Job_JOB-1/Palawan/JOB-1_{nope}_IMG_1234.RAF');
  });

  it('trims empty segments and handles an empty filename template', () => {
    const preview = previewIngestDestination(
      normalizeIngestSettings({ folderTemplate: '/{YYYY}//{MM}/', filenameTemplate: ' ' }),
      new Date(2026, 10, 11),
    );
    expect(preview.folder).toBe('2026/11');
    expect(preview.filename).toBe('');
    expect(preview.relativePath).toBe('2026/11');
  });

  it('uses a real photo name and extension when one is available', () => {
    const preview = previewIngestDestination(
      normalizeIngestSettings({
        folderTemplate: '{YYYY}/{MM}',
        filenameTemplate: '{YYYY}{MM}{DD}_{default_filename}',
      }),
      new Date(2026, 10, 11),
      { stem: 'palawan-112', extension: 'RAF' },
    );
    expect(preview).toEqual({
      folder: '2026/11',
      filename: '20261111_palawan-112.RAF',
      relativePath: '2026/11/20261111_palawan-112.RAF',
    });
  });

  it('falls back to the sample photo when the real name is incomplete', () => {
    expect(previewSampleFromPath('/photos/palawan-112.RAF')).toEqual({
      stem: 'palawan-112',
      extension: 'RAF',
    });
    expect(previewSampleFromPath('/photos/palawan-112')).toBeNull();
    expect(previewSampleFromPath('/photos/folder/')).toBeNull();
    expect(previewSampleFromPath(null)).toBeNull();
  });
});

describe('destination tree', () => {
  function treeEntity(id: string, destinationDir: string, stem: string): IngestEntity {
    return {
      id,
      destinationDir,
      destinationStem: stem,
      primaryPath: `/source/${stem}.CR3`,
      plannedFiles: [
        {
          source: `/source/${stem}.CR3`,
          destination: `${destinationDir}/${stem}.CR3`,
          role: 'raw',
          size: 1,
          primary: true,
          generated: false,
        },
      ],
    } as IngestEntity;
  }

  it('groups photos into sorted destination folders', () => {
    const tree = buildIngestDestinationTree({
      destination: '/destination',
      entities: [
        treeEntity('november', '/destination/2026/11', 'november'),
        treeEntity('october', '/destination/2026/10', 'october'),
        treeEntity('root', '/destination', 'root'),
        treeEntity('nested', '/destination/2026/11/Sub', 'nested'),
      ],
    });

    expect(tree.name).toBe('destination');
    expect(tree.entities.map((entity) => entity.id)).toEqual(['root']);
    expect(tree.folders.map((folder) => folder.name)).toEqual(['2026']);

    const year = tree.folders[0];
    expect(year.folders.map((folder) => folder.name)).toEqual(['10', '11']);
    expect(year.folders[0].entities.map((entity) => entity.id)).toEqual(['october']);
    expect(year.folders[1].entities.map((entity) => entity.id)).toEqual(['november']);
    expect(year.folders[1].folders.map((folder) => folder.name)).toEqual(['Sub']);
    expect(year.folders[1].folders[0].path).toBe('2026/11/Sub');
    expect(year.folders[1].folders[0].entities.map((entity) => entity.id)).toEqual(['nested']);
    expect(ingestDestinationFileName(year.folders[1].entities[0])).toBe('november.CR3');
  });
});

describe('report helpers', () => {
  it('labels capture sources and export names', () => {
    expect(captureSourceLabel('rawExif')).toBe('RAW metadata');
    expect(captureSourceLabel('fileModified')).toBe('File modified time');
    const plan = { createdAt: Date.UTC(2026, 4, 3, 14, 22, 9) } as IngestPlan;
    expect(defaultReportName(plan)).toBe('lumio-ingest-2026-05-03T14-22-09.txt');
  });

  it('builds a complete report with entity, conflict and warning details', () => {
    const plan: IngestPlan = {
      id: 'plan-1',
      createdAt: Date.UTC(2026, 4, 3, 14, 22, 9),
      sources: ['/source'],
      destination: '/destination',
      rules: {
        folderTemplate: '{YYYY}',
        filenameTemplate: '{counter}_{default_filename}',
        jobCode: 'JOB',
        customText: '',
        counterStart: 1,
        counterPadding: 4,
        xmpNaming: 'filenameExt',
        iptc: {
          title: '{default_filename}',
          description: '',
          creator: '',
          copyright: '',
          keywords: [],
        },
      },
      exiftoolPath: null,
      requestedExiftoolPath: null,
      tools: { exiftool: null },
      entities: [
        {
          id: 'e1',
          sourceDir: '/source',
          stem: 'A',
          displayName: 'A.CR3',
          primaryPath: '/source/A.CR3',
          primaryRole: 'raw',
          files: [{ path: '/source/A.CR3', role: 'raw', size: 10, modifiedMs: 1 }],
          capture: {
            display: '2024-05-03 14:22:01',
            timestampMs: 1,
            source: 'rawExif',
            warning: null,
          },
          baseDestinationDir: '/destination/2024',
          baseDestinationStem: '0001_A',
          destinationDir: '/destination/2024',
          destinationStem: '0001_A',
          plannedFiles: [
            {
              source: '/source/A.CR3',
              destination: '/destination/2024/0001_A.CR3',
              role: 'raw',
              size: 10,
              primary: true,
              generated: false,
            },
          ],
          metadataTarget: null,
          status: 'conflict',
          resolution: 'skip',
          conflict: { existing: ['/destination/2024/0001_A.CR3'], message: 'Already exists.' },
          renamed: true,
          renameReason: 'Another photo would use this name.',
          warnings: ['No capture date.'],
          iptc: { title: 'A', description: null, creator: null, copyright: null, keywords: [] },
        },
      ],
      warnings: [{ path: '/source/A.CR3', entityId: 'e1', message: 'No capture date.' }],
      log: [
        { at: '2024-05-03 14:22:01', level: 'info', message: 'Scanned.', path: null, entityId: null },
        {
          at: '2024-05-03 14:22:02',
          level: 'warning',
          message: 'Conflict.',
          path: '/destination',
          entityId: 'e1',
        },
      ],
      summary: {
        entities: 1,
        files: 1,
        bytes: 10,
        conflicts: 1,
        renamed: 1,
        warnings: 1,
        errors: 0,
      },
    };

    const report = buildIngestReport(plan);
    expect(report).toContain('Lumio ingest report');
    expect(report).toContain('Destination: /destination');
    expect(report).toContain('Metadata reader: built-in RAW/JPEG reader');
    expect(report).toContain('1. A.CR3 [conflict]');
    expect(report).toContain('Capture: 2024-05-03 14:22:01 (RAW metadata)');
    expect(report).toContain('Conflict (skip): Already exists.');
    expect(report).toContain('Renamed: Another photo would use this name.');
    expect(report).toContain('[2024-05-03 14:22:02] WARN Conflict.');
    expect(report).toContain('- No capture date. (/source/A.CR3)');
  });
});
