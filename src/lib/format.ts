import type { AssetKind, PreviewSource } from '@/types';

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = value >= 100 || unit === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
});

const shortDateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
});

export function formatDate(ms: number | null | undefined): string {
  if (!ms) return '—';
  try {
    return dateFormatter.format(new Date(ms));
  } catch {
    return '—';
  }
}

export function formatShortDate(ms: number | null | undefined): string {
  if (!ms) return '—';
  try {
    return shortDateFormatter.format(new Date(ms));
  } catch {
    return '—';
  }
}

export function formatCount(count: number): string {
  return new Intl.NumberFormat().format(count);
}

export function formatDimensions(
  width: number | null | undefined,
  height: number | null | undefined,
): string {
  if (!width || !height) return '—';
  return `${formatCount(width)} × ${formatCount(height)}`;
}

export function formatDuration(ms: number | null | undefined): string {
  if (!ms || ms < 0) return '—';
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;
}

function trimNumber(value: number): string {
  const rounded = Number(value.toFixed(1));
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

export function formatShutterSpeed(seconds: number | null | undefined): string {
  if (!seconds || !Number.isFinite(seconds) || seconds <= 0) return '—';
  if (seconds >= 1) return `${trimNumber(seconds)} s`;
  const denominator = Math.round(1 / seconds);
  if (denominator <= 1) return `${trimNumber(seconds)} s`;
  return `1/${denominator} s`;
}

export function formatAperture(value: number | null | undefined): string {
  if (!value || !Number.isFinite(value) || value <= 0) return '—';
  return `f/${trimNumber(value)}`;
}

export function formatFocalLength(value: number | null | undefined): string {
  if (!value || !Number.isFinite(value) || value <= 0) return '—';
  return `${trimNumber(value)} mm`;
}

export function kindLabel(kind: AssetKind, extension?: string): string {
  if (kind === 'raw') return 'RAW';
  if (kind === 'jpeg') return 'JPEG';
  if (kind === 'png') return 'PNG';
  if (kind === 'tiff') return 'TIFF';
  if (kind === 'webp') return 'WebP';
  return extension ? extension.toUpperCase() : 'FILE';
}

export function sourceLabel(source: PreviewSource): string {
  switch (source) {
    case 'original':
      return 'Original image';
    case 'companionJpeg':
      return 'Companion JPEG';
    case 'embeddedRaw':
      return 'Embedded RAW preview';
    case 'placeholder':
    default:
      return 'No preview';
  }
}

export function sourceDescription(source: PreviewSource): string {
  switch (source) {
    case 'original':
      return 'Rendered directly from the displayed image file.';
    case 'companionJpeg':
      return 'Rendered from the JPEG file next to the RAW original.';
    case 'embeddedRaw':
      return 'Extracted from the preview embedded by the camera in the RAW file.';
    case 'placeholder':
    default:
      return 'Lumio could not extract a preview from this file.';
  }
}

export function basename(path: string): string {
  const clean = path.replace(/[\\/]+$/, '');
  const index = Math.max(clean.lastIndexOf('/'), clean.lastIndexOf('\\'));
  return index >= 0 ? clean.slice(index + 1) : clean;
}

export function parentPath(path: string): string {
  const clean = path.replace(/[\\/]+$/, '');
  const index = Math.max(clean.lastIndexOf('/'), clean.lastIndexOf('\\'));
  return index > 0 ? clean.slice(0, index) : clean;
}

/** True when `path` equals `parent` or lives underneath it. */
export function isPathInside(path: string, parent: string): boolean {
  const normalPath = path.replace(/[\\/]+$/, '');
  const normalParent = parent.replace(/[\\/]+$/, '');
  if (normalPath === normalParent) return true;
  return normalPath.startsWith(`${normalParent}/`) || normalPath.startsWith(`${normalParent}\\`);
}

export function pluralize(count: number, singular: string, plural = `${singular}s`): string {
  return count === 1 ? singular : plural;
}
