import { describe, expect, it } from 'vitest';

import {
  bindingFromEvent,
  canonicalBinding,
  eventMatchesBinding,
  formatBindingLabel,
  formatBindingParts,
  matchesShortcut,
  normalizeEventKey,
  normalizeShortcutOverrides,
  resolveShortcutBinding,
  validateShortcutBinding,
} from '@/lib/shortcuts';

function keyEvent(key: string, init: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...init,
  } as KeyboardEvent;
}

describe('shortcut bindings', () => {
  it('normalises event keys across letters, punctuation and special keys', () => {
    expect(normalizeEventKey('G')).toBe('g');
    expect(normalizeEventKey('ArrowLeft')).toBe('arrowleft');
    expect(normalizeEventKey(' ')).toBe('space');
    expect(normalizeEventKey('?')).toBe('question');
    expect(normalizeEventKey('F5')).toBe('f5');
    expect(normalizeEventKey('Dead')).toBeNull();
  });

  it('captures modifiers and ignores shift already encoded in symbols', () => {
    expect(bindingFromEvent(keyEvent('G', { shiftKey: true }))).toBe('shift+g');
    expect(bindingFromEvent(keyEvent('?', { shiftKey: true }))).toBe('question');
    expect(bindingFromEvent(keyEvent('o', { metaKey: true }))).toBe('mod+o');
    expect(bindingFromEvent(keyEvent(' ', { shiftKey: true }))).toBe('shift+space');
    expect(bindingFromEvent(keyEvent('Meta'))).toBeNull();
  });

  it('matches modifiers against the mod abstraction', () => {
    expect(eventMatchesBinding(keyEvent('o', { metaKey: true }), 'mod+o')).toBe(true);
    expect(eventMatchesBinding(keyEvent('o', { ctrlKey: true }), 'mod+o')).toBe(true);
    expect(eventMatchesBinding(keyEvent('o'), 'mod+o')).toBe(false);
    expect(eventMatchesBinding(keyEvent('g', { metaKey: true }), 'g')).toBe(false);
    expect(eventMatchesBinding(keyEvent('G', { shiftKey: true }), 'shift+g')).toBe(true);
    expect(eventMatchesBinding(keyEvent('?', { shiftKey: true }), 'question')).toBe(true);
  });

  it('formats bindings for both platforms', () => {
    expect(formatBindingParts('mod+shift+g', true)).toEqual(['⌘', '⇧', 'G']);
    expect(formatBindingParts('mod+shift+g', false)).toEqual(['Ctrl', 'Shift', 'G']);
    expect(formatBindingLabel('mod+comma', true)).toBe('⌘,');
    expect(formatBindingLabel('arrowleft', true)).toBe('←');
    expect(formatBindingLabel('space', true)).toBe('Space');
  });

  it('resolves overrides and falls back for invalid values', () => {
    expect(resolveShortcutBinding({}, 'view.grid')).toBe('g');
    expect(resolveShortcutBinding({ 'view.grid': 'shift+g' }, 'view.grid')).toBe('shift+g');
    expect(resolveShortcutBinding({ 'view.grid': 'not a key' }, 'view.grid')).toBe('g');
  });

  it('normalises persisted overrides', () => {
    expect(
      normalizeShortcutOverrides({
        'view.grid': 'Shift+G',
        'view.list': 'g', // valid, even if it conflicts; the editor prevents that
        'view.masonry': 'm', // same default; not stored
        unknown: 'x',
        'view.justified': 'not+a+binding',
        42: 'x',
      }),
    ).toEqual({
      'view.grid': 'shift+g',
      'view.list': 'g',
    });
  });

  it('matches primary keys and fixed aliases', () => {
    expect(matchesShortcut(keyEvent('Enter'), 'browse.fullscreen', {})).toBe(true);
    expect(matchesShortcut(keyEvent(' ', { metaKey: true }), 'browse.fullscreen', {})).toBe(false);
    expect(matchesShortcut(keyEvent('Enter'), 'browse.fullscreen', { 'browse.fullscreen': 'f' }))
      .toBe(true);
  });

  it('validates reserved keys, system shortcuts and conflicts', () => {
    expect(validateShortcutBinding('view.grid', 'escape', {})).toMatch(/reserved/);
    expect(validateShortcutBinding('view.grid', 'arrowleft', {})).toMatch(/Arrow keys/);
    expect(validateShortcutBinding('viewer.previous', 'arrowdown', {})).toBeNull();
    expect(validateShortcutBinding('viewer.close', 'delete', {})).toMatch(/Delete/);
    expect(validateShortcutBinding('view.grid', 'mod+q', {})).toMatch(/macOS/);
    expect(validateShortcutBinding('view.grid', 'enter', {})).toMatch(/fullscreen/);
    expect(validateShortcutBinding('view.grid', 'shift+g', {})).toBeNull();
    expect(
      validateShortcutBinding('view.grid', 'shift+g', { 'view.list': 'shift+g' }),
    ).toMatch(/List view/);
    // Same key is fine in a different context.
    expect(validateShortcutBinding('viewer.next', 'shift+g', { 'view.grid': 'shift+g' })).toBeNull();
  });

  it('canonicalises duplicate modifiers', () => {
    expect(canonicalBinding('shift+shift+g')).toBe('shift+g');
    expect(canonicalBinding('meta+g')).toBeNull();
  });
});
