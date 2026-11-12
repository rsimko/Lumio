export type FontKind = 'ui' | 'mono';

export interface FontPreset {
  value: string;
  label: string;
  description: string;
}

export const UI_FONT_PRESETS: FontPreset[] = [
  {
    value: 'avenir',
    label: 'Avenir',
    description: 'Default',
  },
  {
    value: 'system',
    label: 'System UI',
    description: 'macOS',
  },
];

export const MONO_FONT_PRESETS: FontPreset[] = [
  {
    value: 'jetbrains',
    label: 'JetBrains Mono',
    description: 'Default',
  },
  {
    value: 'system',
    label: 'System Mono',
    description: 'macOS',
  },
];

const UI_FALLBACK_STACK =
  "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Inter, Roboto, 'Helvetica Neue', Arial, sans-serif";
const MONO_FALLBACK_STACK =
  "'SF Mono', ui-monospace, Menlo, Monaco, Consolas, monospace";

const PRESET_STACKS: Record<string, Record<string, string>> = {
  ui: {
    avenir: `'Avenir Next', Avenir, ${UI_FALLBACK_STACK}`,
    system: UI_FALLBACK_STACK,
  },
  mono: {
    jetbrains: `'JetBrains Mono Variable', 'JetBrains Mono', ${MONO_FALLBACK_STACK}`,
    system: MONO_FALLBACK_STACK,
  },
};

function quoteFontFamily(family: string): string {
  const escaped = family.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
  return `'${escaped}'`;
}

/** Remove characters that cannot be part of a CSS font-family declaration. */
export function sanitizeFontFamily(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const cleaned = value
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .trim()
    .slice(0, 128);
  return cleaned ? cleaned : null;
}

export function fontChoiceLabel(choice: string, kind: FontKind): string {
  const presets = kind === 'ui' ? UI_FONT_PRESETS : MONO_FONT_PRESETS;
  return presets.find((preset) => preset.value === choice)?.label ?? choice;
}

/** Resolve friendly preset labels back to their stable stored tokens. */
export function resolveFontChoice(value: string, kind: FontKind): string | null {
  const cleaned = sanitizeFontFamily(value);
  if (!cleaned) return null;
  const presets = kind === 'ui' ? UI_FONT_PRESETS : MONO_FONT_PRESETS;
  const preset = presets.find(
    (candidate) =>
      candidate.value.toLowerCase() === cleaned.toLowerCase() ||
      candidate.label.toLowerCase() === cleaned.toLowerCase(),
  );
  return preset?.value ?? cleaned;
}

/** Build a CSS stack that keeps the bundled defaults and system fallbacks. */
export function fontFamilyStack(choice: string, kind: FontKind): string {
  const presetStack = PRESET_STACKS[kind][choice];
  if (presetStack) return presetStack;
  const fallback = kind === 'ui' ? UI_FALLBACK_STACK : MONO_FALLBACK_STACK;
  return `${quoteFontFamily(choice)}, ${fallback}`;
}
