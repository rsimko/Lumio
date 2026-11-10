import { describe, expect, it } from 'vitest';

import {
  editorNameFromPath,
  isApplicationPath,
  normalizeDefaultEditorPath,
  normalizeExternalEditors,
} from '@/lib/editors';

describe('external editors', () => {
  it('derives display names from application bundles', () => {
    expect(editorNameFromPath('/Applications/Adobe Photoshop 2024.app')).toBe(
      'Adobe Photoshop 2024',
    );
    expect(editorNameFromPath('/Applications/Capture One.app/')).toBe('Capture One');
    expect(editorNameFromPath('/')).toBe('Application');
  });

  it('recognises application bundle paths', () => {
    expect(isApplicationPath('/Applications/DxO PhotoLab.app')).toBe(true);
    expect(isApplicationPath('/Applications/Luminar.app/')).toBe(true);
    expect(isApplicationPath('/Applications/notes.txt')).toBe(false);
  });

  it('normalises and deduplicates persisted editors', () => {
    expect(
      normalizeExternalEditors([
        { name: ' Photoshop ', path: '/Applications/Photoshop.app/' },
        { path: '/Applications/Photoshop.app' },
        { name: '', path: '/Applications/Lightroom.app' },
        { name: 'Broken' },
        null,
        'nope',
      ]),
    ).toEqual([
      { name: 'Photoshop', path: '/Applications/Photoshop.app' },
      { name: 'Lightroom', path: '/Applications/Lightroom.app' },
    ]);
  });

  it('falls back to the first editor when the default is missing', () => {
    const editors = [
      { name: 'Photoshop', path: '/Applications/Photoshop.app' },
      { name: 'Lightroom', path: '/Applications/Lightroom.app' },
    ];
    expect(normalizeDefaultEditorPath('/Applications/Lightroom.app', editors)).toBe(
      '/Applications/Lightroom.app',
    );
    expect(normalizeDefaultEditorPath('/Applications/Gone.app', editors)).toBe(
      '/Applications/Photoshop.app',
    );
    expect(normalizeDefaultEditorPath(null, [])).toBeNull();
  });
});
