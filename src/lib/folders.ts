import { isPathInside } from '@/lib/format';
import type { FolderRemovalTarget } from '@/types';

function normalizedPath(path: string): string {
  return path.replace(/[\\/]+$/, '');
}

/**
 * Collapse a folder-removal request so no folder is removed twice and nested
 * folders are covered by their selected ancestor. The result keeps the order
 * in which the folders were selected.
 */
export function pruneFolderRemovalTargets(
  targets: readonly FolderRemovalTarget[],
): FolderRemovalTarget[] {
  const unique = new Map<string, FolderRemovalTarget>();
  for (const target of targets) {
    const key = normalizedPath(target.path);
    if (!unique.has(key)) unique.set(key, target);
  }
  const values = [...unique.values()];
  return values.filter(
    (target) =>
      !values.some(
        (other) =>
          normalizedPath(other.path) !== normalizedPath(target.path) &&
          isPathInside(target.path, other.path),
      ),
  );
}
