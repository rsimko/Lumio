import { describe, expect, it } from 'vitest';

import {
  basename,
  formatAperture,
  formatBytes,
  formatDimensions,
  formatDuration,
  formatFocalLength,
  formatShutterSpeed,
  isPathInside,
  kindLabel,
  parentPath,
  previewSourceBadge,
  sourceDescription,
  sourceLabel,
} from '@/lib/format';

describe('formatBytes', () => {
  it('uses human readable units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(52_428_800)).toBe('50.0 MB');
    expect(formatBytes(3_221_225_472)).toBe('3.0 GB');
  });

  it('survives invalid input', () => {
    expect(formatBytes(Number.NaN)).toBe('0 B');
    expect(formatBytes(-10)).toBe('0 B');
  });
});

describe('format helpers', () => {
  it('formats dimensions and durations', () => {
    expect(formatDimensions(6000, 4000)).toBe('6,000 × 4,000');
    expect(formatDimensions(null, 4000)).toBe('—');
    expect(formatDuration(420)).toBe('420 ms');
    expect(formatDuration(2500)).toBe('2.5 s');
    expect(formatDuration(12_400)).toBe('12 s');
  });

  it('formats camera exposure values', () => {
    expect(formatShutterSpeed(1 / 250)).toBe('1/250 s');
    expect(formatShutterSpeed(2)).toBe('2 s');
    expect(formatShutterSpeed(1.5)).toBe('1.5 s');
    expect(formatShutterSpeed(0.8)).toBe('0.8 s');
    expect(formatShutterSpeed(0)).toBe('—');
    expect(formatAperture(2.8)).toBe('f/2.8');
    expect(formatAperture(null)).toBe('—');
    expect(formatFocalLength(35)).toBe('35 mm');
    expect(formatFocalLength(35.5)).toBe('35.5 mm');
    expect(formatFocalLength(0)).toBe('—');
  });

  it('labels asset kinds and preview sources', () => {
    expect(kindLabel('raw', 'cr3')).toBe('RAW');
    expect(kindLabel('jpeg', 'jpg')).toBe('JPEG');
    expect(kindLabel('other', 'heic')).toBe('HEIC');
    expect(sourceLabel('embeddedRaw')).toBe('Embedded RAW preview');
    expect(sourceLabel('companionJpeg')).toBe('Companion JPEG');
    expect(sourceLabel('placeholder')).toBe('No preview');
    expect(sourceDescription('original')).toContain('displayed image');
  });

  it('uses short preview badge labels', () => {
    expect(previewSourceBadge('embeddedRaw')).toBe('EMBEDDED');
    expect(previewSourceBadge('companionJpeg')).toBe('JPG');
    expect(previewSourceBadge('original', 'raw')).toBe('RAW');
    expect(previewSourceBadge('original', 'jpeg')).toBe('JPG');
    expect(previewSourceBadge('original', 'tiff')).toBe('TIFF');
    expect(previewSourceBadge('placeholder')).toBe('NONE');
  });

  it('handles POSIX paths', () => {
    expect(basename('/photos/2026/canon.CR3')).toBe('canon.CR3');
    expect(basename('/photos/2026/')).toBe('2026');
    expect(parentPath('/photos/2026/canon.CR3')).toBe('/photos/2026');
    expect(isPathInside('/photos/2026/canon.CR3', '/photos')).toBe(true);
    expect(isPathInside('/photos', '/photos')).toBe(true);
    expect(isPathInside('/photography', '/photos')).toBe(false);
  });
});
