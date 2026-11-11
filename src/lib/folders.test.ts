import { describe, expect, it } from 'vitest';

import { pruneFolderRemovalTargets } from '@/lib/folders';
import type { FolderRemovalTarget } from '@/types';

function target(path: string, kind: FolderRemovalTarget['kind'] = 'folder'): FolderRemovalTarget {
  return { kind, rootId: 'root-1', name: path.split('/').at(-1) ?? path, path };
}

describe('pruneFolderRemovalTargets', () => {
  it('removes duplicate paths', () => {
    const result = pruneFolderRemovalTargets([
      target('/library/2026'),
      target('/library/2026'),
      target('/library/2025'),
    ]);
    expect(result.map((entry) => entry.path)).toEqual(['/library/2026', '/library/2025']);
  });

  it('drops nested folders covered by a selected ancestor', () => {
    const result = pruneFolderRemovalTargets([
      target('/library/2026'),
      target('/library/2026/11'),
      target('/library/2025'),
    ]);
    expect(result.map((entry) => entry.path)).toEqual(['/library/2026', '/library/2025']);
  });

  it('keeps a root and drops folders selected inside it', () => {
    const result = pruneFolderRemovalTargets([
      target('/library', 'root'),
      target('/library/2026/11'),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0]?.kind).toBe('root');
  });

  it('treats trailing slashes as the same folder', () => {
    const result = pruneFolderRemovalTargets([target('/library/2026/'), target('/library/2026')]);
    expect(result).toHaveLength(1);
  });

  it('keeps siblings from the same root', () => {
    const result = pruneFolderRemovalTargets([
      target('/library/2026/11'),
      target('/library/2026/12'),
    ]);
    expect(result.map((entry) => entry.path)).toEqual([
      '/library/2026/11',
      '/library/2026/12',
    ]);
  });
});
