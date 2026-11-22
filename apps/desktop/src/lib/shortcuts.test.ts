import { describe, expect, it } from 'vitest';

import { shortcutFor } from './shortcuts';

const press = (key: string, modifiers: Partial<KeyboardEvent> = {}) =>
  shortcutFor({
    key,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...modifiers,
  });

describe('shortcutFor', () => {
  it('maps Ctrl and Cmd combinations to actions', () => {
    expect(press('Enter', { ctrlKey: true })).toBe('send');
    expect(press('Enter', { metaKey: true })).toBe('send');
    expect(press('s', { ctrlKey: true })).toBe('save');
    expect(press('S', { metaKey: true })).toBe('save');
    expect(press('n', { ctrlKey: true })).toBe('newRequest');
    expect(press('e', { ctrlKey: true })).toBe('environment');
    expect(press('k', { ctrlKey: true })).toBe('search');
  });

  it('ignores other keys and modifiers', () => {
    expect(press('Enter')).toBeNull();
    expect(press('s')).toBeNull();
    expect(press('x', { ctrlKey: true })).toBeNull();
    expect(press('n', { ctrlKey: true, shiftKey: true })).toBeNull();
    expect(press('e', { ctrlKey: true, altKey: true })).toBeNull();
    // Shift+Ctrl+Enter still sends.
    expect(press('Enter', { ctrlKey: true, shiftKey: true })).toBe('send');
  });
});
