import { describe, expect, it, vi } from 'vitest';

import { entry, fit, isNavigationKey, separator, step } from './menu';

describe('menus', () => {
  it('build entries with an action or a submenu', () => {
    const act = vi.fn();
    expect(entry('a', 'A', act, 'Ctrl+A')).toEqual({
      kind: 'entry',
      id: 'a',
      label: 'A',
      onSelect: act,
      shortcut: 'Ctrl+A',
    });
    const sub = entry('s', 'S', [separator]);
    expect(sub).toEqual({ kind: 'entry', id: 's', label: 'S', items: [separator] });
    expect('shortcut' in sub).toBe(false);
  });

  it('move the focus with wrapping arrows, Home and End', () => {
    expect(step(0, 'ArrowDown', 3)).toBe(1);
    expect(step(2, 'ArrowDown', 3)).toBe(0);
    expect(step(-1, 'ArrowDown', 3)).toBe(0);
    expect(step(0, 'ArrowUp', 3)).toBe(2);
    expect(step(1, 'Home', 3)).toBe(0);
    expect(step(0, 'End', 3)).toBe(2);
    expect(step(0, 'End', 0)).toBe(-1);
    expect(isNavigationKey('Home')).toBe(true);
    expect(isNavigationKey('Enter')).toBe(false);
  });

  it('stay inside the window', () => {
    const view = { width: 800, height: 600 };
    expect(fit(10, 20, 200, 100, view)).toEqual({ left: 10, top: 20 });
    expect(fit(700, 550, 200, 100, view)).toEqual({ left: 596, top: 496 });
    expect(fit(-5, -5, 200, 100, view)).toEqual({ left: 4, top: 4 });
  });
});
