/**
 * Editable keyboard shortcuts.
 *
 * Bindings are stored as canonical strings such as `mod+shift+g` or `question`.
 * The `mod` modifier maps to Command on macOS and Control elsewhere. Keys are
 * normalised names rather than raw `event.key` values so digits, letters,
 * punctuation and special keys can all round-trip through JSON settings.
 */

export type ShortcutContext = 'browse' | 'viewer';

export type ShortcutGroup = 'Browsing' | 'Metadata' | 'Library' | 'Fullscreen';

export type ShortcutId =
  | 'browse.fullscreen'
  | 'browse.editDefault'
  | 'browse.delete'
  | 'browse.thumbLarger'
  | 'browse.thumbSmaller'
  | 'browse.thumbReset'
  | 'metadata.rating1'
  | 'metadata.rating2'
  | 'metadata.rating3'
  | 'metadata.rating4'
  | 'metadata.rating5'
  | 'metadata.pick'
  | 'metadata.reject'
  | 'metadata.unflag'
  | 'metadata.favorite'
  | 'view.grid'
  | 'view.masonry'
  | 'view.justified'
  | 'view.list'
  | 'view.filmstrip'
  | 'library.addFolder'
  | 'library.refresh'
  | 'library.selectAll'
  | 'library.recursive'
  | 'library.filmstrip'
  | 'library.info'
  | 'library.theme'
  | 'library.settings'
  | 'help.shortcuts'
  | 'viewer.previous'
  | 'viewer.next'
  | 'viewer.close';

export interface ShortcutDefinition {
  id: ShortcutId;
  label: string;
  group: ShortcutGroup;
  context: ShortcutContext;
  /** Canonical primary binding. */
  binding: string;
  /** Additional fixed keys that always trigger the same action. */
  aliases?: string[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  'Browsing',
  'Metadata',
  'Library',
  'Fullscreen',
];

export const SHORTCUT_DEFINITIONS: readonly ShortcutDefinition[] = [
  {
    id: 'browse.fullscreen',
    label: 'Open fullscreen viewer',
    group: 'Browsing',
    context: 'browse',
    binding: 'space',
    aliases: ['enter'],
  },
  {
    id: 'browse.editDefault',
    label: 'Edit with default application',
    group: 'Browsing',
    context: 'browse',
    binding: 'mod+e',
  },
  {
    id: 'browse.delete',
    label: 'Delete selected photos',
    group: 'Browsing',
    context: 'browse',
    binding: 'delete',
    aliases: ['backspace'],
  },
  {
    id: 'browse.thumbLarger',
    label: 'Increase thumbnail size',
    group: 'Browsing',
    context: 'browse',
    binding: 'plus',
    aliases: ['equal'],
  },
  {
    id: 'browse.thumbSmaller',
    label: 'Decrease thumbnail size',
    group: 'Browsing',
    context: 'browse',
    binding: 'minus',
    aliases: ['underscore'],
  },
  {
    id: 'browse.thumbReset',
    label: 'Reset thumbnail size',
    group: 'Browsing',
    context: 'browse',
    binding: '0',
  },
  {
    id: 'metadata.rating1',
    label: 'Rate 1 star',
    group: 'Metadata',
    context: 'browse',
    binding: '1',
  },
  {
    id: 'metadata.rating2',
    label: 'Rate 2 stars',
    group: 'Metadata',
    context: 'browse',
    binding: '2',
  },
  {
    id: 'metadata.rating3',
    label: 'Rate 3 stars',
    group: 'Metadata',
    context: 'browse',
    binding: '3',
  },
  {
    id: 'metadata.rating4',
    label: 'Rate 4 stars',
    group: 'Metadata',
    context: 'browse',
    binding: '4',
  },
  {
    id: 'metadata.rating5',
    label: 'Rate 5 stars',
    group: 'Metadata',
    context: 'browse',
    binding: '5',
  },
  {
    id: 'metadata.pick',
    label: 'Flag as picked',
    group: 'Metadata',
    context: 'browse',
    binding: 'p',
  },
  {
    id: 'metadata.reject',
    label: 'Flag as rejected',
    group: 'Metadata',
    context: 'browse',
    binding: 'x',
  },
  {
    id: 'metadata.unflag',
    label: 'Remove cull flag',
    group: 'Metadata',
    context: 'browse',
    binding: 'u',
  },
  {
    id: 'metadata.favorite',
    label: 'Toggle favorite',
    group: 'Metadata',
    context: 'browse',
    binding: 'shift+f',
  },
  { id: 'view.grid', label: 'Grid view', group: 'Browsing', context: 'browse', binding: 'g' },
  {
    id: 'view.masonry',
    label: 'Masonry view',
    group: 'Browsing',
    context: 'browse',
    binding: 'm',
  },
  {
    id: 'view.justified',
    label: 'Justified view',
    group: 'Browsing',
    context: 'browse',
    binding: 'j',
  },
  { id: 'view.list', label: 'List view', group: 'Browsing', context: 'browse', binding: 'l' },
  {
    id: 'view.filmstrip',
    label: 'Filmstrip view',
    group: 'Browsing',
    context: 'browse',
    binding: 'f',
  },
  {
    id: 'library.addFolder',
    label: 'Add a folder',
    group: 'Library',
    context: 'browse',
    binding: 'mod+o',
  },
  {
    id: 'library.refresh',
    label: 'Refresh selected folder',
    group: 'Library',
    context: 'browse',
    binding: 'mod+r',
  },
  {
    id: 'library.selectAll',
    label: 'Select all photos',
    group: 'Library',
    context: 'browse',
    binding: 'mod+a',
  },
  {
    id: 'library.recursive',
    label: 'Toggle include subfolders',
    group: 'Library',
    context: 'browse',
    binding: 'r',
  },
  {
    id: 'library.filmstrip',
    label: 'Toggle filmstrip',
    group: 'Library',
    context: 'browse',
    binding: 'b',
  },
  {
    id: 'library.info',
    label: 'Toggle info panel',
    group: 'Library',
    context: 'browse',
    binding: 'i',
  },
  {
    id: 'library.theme',
    label: 'Toggle light / dark appearance',
    group: 'Library',
    context: 'browse',
    binding: 't',
  },
  {
    id: 'library.settings',
    label: 'Open settings',
    group: 'Library',
    context: 'browse',
    binding: 'mod+comma',
  },
  {
    id: 'help.shortcuts',
    label: 'Show keyboard shortcuts',
    group: 'Library',
    context: 'browse',
    binding: 'question',
  },
  {
    id: 'viewer.previous',
    label: 'Previous photo',
    group: 'Fullscreen',
    context: 'viewer',
    binding: 'arrowleft',
  },
  {
    id: 'viewer.next',
    label: 'Next photo',
    group: 'Fullscreen',
    context: 'viewer',
    binding: 'arrowright',
  },
  {
    id: 'viewer.close',
    label: 'Exit fullscreen',
    group: 'Fullscreen',
    context: 'viewer',
    binding: 'space',
    aliases: ['enter'],
  },
];

export const DEFAULT_SHORTCUTS: Record<ShortcutId, string> = Object.fromEntries(
  SHORTCUT_DEFINITIONS.map((definition) => [definition.id, definition.binding]),
) as Record<ShortcutId, string>;

export const SHORTCUT_BY_ID: Record<ShortcutId, ShortcutDefinition> = Object.fromEntries(
  SHORTCUT_DEFINITIONS.map((definition) => [definition.id, definition]),
) as Record<ShortcutId, ShortcutDefinition>;

export type ShortcutOverrides = Record<string, string>;

export interface ParsedBinding {
  key: string;
  mod: boolean;
  alt: boolean;
  shift: boolean;
}

const NAMED_EVENT_KEYS: Record<string, string> = {
  ' ': 'space',
  Spacebar: 'space',
  Enter: 'enter',
  Escape: 'escape',
  Esc: 'escape',
  Tab: 'tab',
  Delete: 'delete',
  Backspace: 'backspace',
  ArrowLeft: 'arrowleft',
  ArrowRight: 'arrowright',
  ArrowUp: 'arrowup',
  ArrowDown: 'arrowdown',
  Home: 'home',
  End: 'end',
  PageUp: 'pageup',
  PageDown: 'pagedown',
  Insert: 'insert',
};

const PUNCTUATION_KEYS: Record<string, string> = {
  '+': 'plus',
  '-': 'minus',
  '=': 'equal',
  ',': 'comma',
  '.': 'period',
  '/': 'slash',
  '?': 'question',
  '\\': 'backslash',
  ';': 'semicolon',
  "'": 'quote',
  '`': 'backtick',
  '[': 'bracketleft',
  ']': 'bracketright',
  _: 'underscore',
  '!': 'exclam',
  '@': 'at',
  '#': 'hash',
  $: 'dollar',
  '%': 'percent',
  '^': 'caret',
  '&': 'ampersand',
  '*': 'asterisk',
  '(': 'parenleft',
  ')': 'parenright',
  '<': 'less',
  '>': 'greater',
  '|': 'bar',
  ':': 'colon',
  '"': 'quotedbl',
  '~': 'tilde',
};

const KEY_LABELS: Record<string, string> = {
  space: 'Space',
  enter: 'Enter',
  escape: 'Esc',
  tab: 'Tab',
  delete: 'Delete',
  backspace: 'Backspace',
  arrowleft: '←',
  arrowright: '→',
  arrowup: '↑',
  arrowdown: '↓',
  home: 'Home',
  end: 'End',
  pageup: 'Page Up',
  pagedown: 'Page Down',
  insert: 'Insert',
  plus: '+',
  minus: '−',
  equal: '=',
  comma: ',',
  period: '.',
  slash: '/',
  question: '?',
  backslash: '\\',
  semicolon: ';',
  quote: "'",
  backtick: '`',
  bracketleft: '[',
  bracketright: ']',
  underscore: '_',
  exclam: '!',
  at: '@',
  hash: '#',
  dollar: '$',
  percent: '%',
  caret: '^',
  ampersand: '&',
  asterisk: '*',
  parenleft: '(',
  parenright: ')',
  less: '<',
  greater: '>',
  bar: '|',
  colon: ':',
  quotedbl: '"',
  tilde: '~',
};

const MODIFIER_KEYS = new Set(['Meta', 'Control', 'Alt', 'Shift', 'CapsLock', 'OS']);

const ARROW_KEYS = new Set(['arrowleft', 'arrowright', 'arrowup', 'arrowdown']);

/** Keys whose Shift state is not already encoded in `event.key`. */
const SHIFT_SENSITIVE_KEYS = new Set([
  ...ARROW_KEYS,
  'space',
  'enter',
  'escape',
  'tab',
  'delete',
  'backspace',
  'home',
  'end',
  'pageup',
  'pagedown',
  'insert',
]);

const RESERVED_SYSTEM_KEYS = new Set(['q', 'w', 'm', 'h', 'space', 'tab']);

export function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return true;
  return /Mac|iPhone|iPad/.test(navigator.platform);
}

export function isShortcutId(value: string): value is ShortcutId {
  return Object.prototype.hasOwnProperty.call(SHORTCUT_BY_ID, value);
}

export function normalizeEventKey(key: string): string | null {
  const named = NAMED_EVENT_KEYS[key];
  if (named) return named;
  const punctuation = PUNCTUATION_KEYS[key];
  if (punctuation) return punctuation;
  if (/^[a-zA-Z]$/.test(key)) return key.toLowerCase();
  if (/^[0-9]$/.test(key)) return key;
  if (/^F\d{1,2}$/.test(key)) return key.toLowerCase();
  return null;
}

function isShiftSensitive(key: string): boolean {
  return /^[a-z]$/.test(key) || SHIFT_SENSITIVE_KEYS.has(key) || /^f\d+$/.test(key);
}

function isKnownKey(key: string): boolean {
  return (
    Object.prototype.hasOwnProperty.call(KEY_LABELS, key) ||
    /^[a-z]$/.test(key) ||
    /^[0-9]$/.test(key) ||
    /^f\d{1,2}$/.test(key)
  );
}

export function parseBinding(binding: string): ParsedBinding | null {
  if (!binding) return null;
  const parts = binding.trim().toLowerCase().split('+');
  const key = parts.pop();
  if (!key || !isKnownKey(key)) return null;
  let mod = false;
  let alt = false;
  let shift = false;
  for (const part of parts) {
    if (part === 'mod') mod = true;
    else if (part === 'alt') alt = true;
    else if (part === 'shift') shift = true;
    else return null;
  }
  return { key, mod, alt, shift };
}

export function serializeBinding(parsed: ParsedBinding): string {
  const parts: string[] = [];
  if (parsed.mod) parts.push('mod');
  if (parsed.alt) parts.push('alt');
  if (parsed.shift) parts.push('shift');
  parts.push(parsed.key);
  return parts.join('+');
}

/** Canonical form used for comparisons and persistence. */
export function canonicalBinding(binding: string): string | null {
  const parsed = parseBinding(binding);
  return parsed ? serializeBinding(parsed) : null;
}

export function bindingFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;
  const key = normalizeEventKey(event.key);
  if (!key) return null;
  return serializeBinding({
    key,
    mod: event.metaKey || event.ctrlKey,
    alt: event.altKey,
    // For punctuation and digits, `event.key` already contains the shifted
    // symbol; recording Shift as well would produce impossible bindings.
    shift: event.shiftKey && isShiftSensitive(key),
  });
}

export function eventMatchesBinding(event: KeyboardEvent, binding: string): boolean {
  const parsed = parseBinding(binding);
  if (!parsed) return false;
  const key = normalizeEventKey(event.key);
  if (!key || key !== parsed.key) return false;
  if ((event.metaKey || event.ctrlKey) !== parsed.mod) return false;
  if (event.altKey !== parsed.alt) return false;
  if (isShiftSensitive(parsed.key) && event.shiftKey !== parsed.shift) return false;
  return true;
}

export function resolveShortcutBinding(overrides: ShortcutOverrides, id: ShortcutId): string {
  const override = overrides[id];
  return override && parseBinding(override) ? override : DEFAULT_SHORTCUTS[id];
}

export function matchesShortcut(
  event: KeyboardEvent,
  id: ShortcutId,
  overrides: ShortcutOverrides,
): boolean {
  const definition = SHORTCUT_BY_ID[id];
  if (eventMatchesBinding(event, resolveShortcutBinding(overrides, id))) return true;
  return definition.aliases?.some((alias) => eventMatchesBinding(event, alias)) ?? false;
}

export function keyLabel(key: string): string {
  const label = KEY_LABELS[key];
  if (label) return label;
  if (/^f\d+$/.test(key)) return key.toUpperCase();
  if (key.length === 1) return key.toUpperCase();
  return key;
}

export function formatBindingParts(binding: string, mac = isMacPlatform()): string[] {
  const parsed = parseBinding(binding);
  if (!parsed) return [binding];
  const parts: string[] = [];
  if (parsed.mod) parts.push(mac ? '⌘' : 'Ctrl');
  if (parsed.alt) parts.push(mac ? '⌥' : 'Alt');
  if (parsed.shift) parts.push(mac ? '⇧' : 'Shift');
  parts.push(keyLabel(parsed.key));
  return parts;
}

export function formatBindingLabel(binding: string, mac = isMacPlatform()): string {
  const parts = formatBindingParts(binding, mac);
  return mac ? parts.join('') : parts.join('+');
}

/** Only keeps known actions with valid, canonical bindings. */
export function normalizeShortcutOverrides(value: unknown): ShortcutOverrides {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  const overrides: ShortcutOverrides = {};
  for (const [id, raw] of Object.entries(value as Record<string, unknown>)) {
    if (!isShortcutId(id) || typeof raw !== 'string') continue;
    const canonical = canonicalBinding(raw);
    if (!canonical) continue;
    if (canonical !== DEFAULT_SHORTCUTS[id]) overrides[id] = canonical;
  }
  return overrides;
}

/**
 * Returns a human-readable conflict message, or null when the binding can be
 * assigned to the action. Actions in different contexts may share a key (for
 * example Space opens the viewer and also closes it in fullscreen).
 */
export function validateShortcutBinding(
  id: ShortcutId,
  binding: string,
  overrides: ShortcutOverrides,
): string | null {
  const definition = SHORTCUT_BY_ID[id];
  const parsed = parseBinding(binding);
  if (!parsed) return 'That key combination cannot be used.';
  if (parsed.key === 'escape' || parsed.key === 'tab') {
    return `${keyLabel(parsed.key)} is reserved.`;
  }
  if (definition.context === 'browse' && !parsed.mod && ARROW_KEYS.has(parsed.key)) {
    return 'Arrow keys are reserved for moving the selection.';
  }
  if (
    definition.context === 'viewer' &&
    !parsed.mod &&
    (parsed.key === 'delete' || parsed.key === 'backspace')
  ) {
    return 'Delete is reserved for removing photos in fullscreen.';
  }
  if (parsed.mod && RESERVED_SYSTEM_KEYS.has(parsed.key)) {
    return 'That shortcut is reserved by macOS.';
  }

  const canonical = serializeBinding(parsed);
  for (const other of SHORTCUT_DEFINITIONS) {
    if (other.id === id || other.context !== definition.context) continue;
    const bindings = [resolveShortcutBinding(overrides, other.id), ...(other.aliases ?? [])];
    if (bindings.some((candidate) => canonicalBinding(candidate) === canonical)) {
      return `Already used for “${other.label}”.`;
    }
  }
  return null;
}
