import { describe, expect, it } from 'vitest';

import {
  fontChoiceLabel,
  fontFamilyStack,
  resolveFontChoice,
  sanitizeFontFamily,
} from '@/lib/fonts';

describe('font settings', () => {
  it('keeps the bundled defaults first in their fallback stacks', () => {
    expect(fontFamilyStack('avenir', 'ui')).toMatch(/^'Avenir Next', Avenir,/);
    expect(fontFamilyStack('jetbrains', 'mono')).toMatch(
      /^'JetBrains Mono Variable', 'JetBrains Mono',/,
    );
    expect(fontFamilyStack('system', 'ui')).toMatch(/^-apple-system/);
  });

  it('quotes arbitrary installed font families safely', () => {
    expect(fontFamilyStack("Photographer's Sans", 'ui')).toMatch(
      /^'Photographer\\'s Sans', -apple-system/,
    );
    expect(fontFamilyStack('Fira Code', 'mono')).toMatch(
      /^'Fira Code', 'SF Mono', ui-monospace/,
    );
  });

  it('resolves preset labels and sanitizes custom input', () => {
    expect(resolveFontChoice('System UI', 'ui')).toBe('system');
    expect(resolveFontChoice('JetBrains Mono', 'mono')).toBe('jetbrains');
    expect(resolveFontChoice('  Fira Code\n', 'mono')).toBe('Fira Code');
    expect(resolveFontChoice('   ', 'ui')).toBeNull();
    expect(fontChoiceLabel('avenir', 'ui')).toBe('Avenir');
    expect(fontChoiceLabel('Fira Code', 'mono')).toBe('Fira Code');
  });

  it('removes control characters and caps family length', () => {
    expect(sanitizeFontFamily(' Bad\nFont ')).toBe('BadFont');
    expect(sanitizeFontFamily('x'.repeat(140))).toHaveLength(128);
    expect(sanitizeFontFamily('\u0000')).toBeNull();
    expect(sanitizeFontFamily(undefined)).toBeNull();
  });
});
