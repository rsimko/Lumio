import { basename } from '@/lib/format';
import type { ExternalEditor } from '@/types';

/** Turns `/Applications/Adobe Photoshop 2024.app` into `Adobe Photoshop 2024`. */
export function editorNameFromPath(path: string): string {
  const name = basename(path).replace(/\.app$/i, '').trim();
  return name || 'Application';
}

/** True for macOS application bundles selected through the file picker. */
export function isApplicationPath(path: string): boolean {
  return /\.app$/i.test(path.trim().replace(/[\\/]+$/, ''));
}

/** Drops malformed and duplicate persisted editor entries. */
export function normalizeExternalEditors(value: unknown): ExternalEditor[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const editors: ExternalEditor[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue;
    const candidate = entry as Partial<ExternalEditor>;
    if (typeof candidate.path !== 'string') continue;
    const path = candidate.path.trim().replace(/[\\/]+$/, '');
    if (!path || seen.has(path)) continue;
    const name =
      typeof candidate.name === 'string' && candidate.name.trim()
        ? candidate.name.trim()
        : editorNameFromPath(path);
    seen.add(path);
    editors.push({ name, path });
  }
  return editors;
}

/**
 * Keeps the configured default when it still exists; otherwise the first
 * editor becomes the default so the Edit shortcut always has a target.
 */
export function normalizeDefaultEditorPath(
  value: unknown,
  editors: readonly ExternalEditor[],
): string | null {
  if (typeof value === 'string' && editors.some((editor) => editor.path === value)) {
    return value;
  }
  return editors[0]?.path ?? null;
}
