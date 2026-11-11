import { describe, expect, it } from 'vitest';

import {
  DEFAULT_INGEST_SETTINGS,
  buildIngestReport,
  captureSourceLabel,
  defaultReportName,
  hasIptcValues,
  ingestRulesFromSettings,
  insertIngestToken,
  normalizeIngestSettings,
  validateIngestSettings,
  validateIngestTemplate,
  xmpNamingExample,
  xmpNamingLabel,
} from '@/lib/ingest';
import type { IngestPlan } from '@/types';

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
    expect(problems.map((problem) => problem.kind)).toContain('text');
  });
});

describe('token insertion', () => {
  it('inserts at the caret and moves the caret after the token', () => {
    expect(insertIngestToken('AB', 'YYYY', 1, 1)).toEqual({
      value: 'A{YYYY}B',
      caret: 7,
    });
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
