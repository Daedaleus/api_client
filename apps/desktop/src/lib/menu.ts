// Menus: their entries and keyboard navigation (WAI-ARIA menu pattern).

/** One entry of a menu: an action, or a submenu if `items` is set. */
export interface MenuEntry {
  kind: 'entry';
  /** Stable id, e.g. for tests (`data-item`). */
  id: string;
  label: string;
  /** Key combination shown next to the label. */
  shortcut?: string;
  /** Opens a submenu instead of acting. */
  items?: MenuItem[];
  onSelect?: () => void;
}

/** A line between groups of entries. */
export interface MenuSeparator {
  kind: 'separator';
}

export type MenuItem = MenuEntry | MenuSeparator;

export const separator: MenuSeparator = { kind: 'separator' };

/** An entry; `action` is a function or a submenu. */
export function entry(
  id: string,
  label: string,
  action: (() => void) | MenuItem[],
  shortcut?: string,
): MenuEntry {
  const made: MenuEntry =
    typeof action === 'function'
      ? { kind: 'entry', id, label, onSelect: action }
      : { kind: 'entry', id, label, items: action };
  if (shortcut !== undefined) made.shortcut = shortcut;
  return made;
}

/** Keys that move the focus within a menu. */
export type NavigationKey = 'ArrowDown' | 'ArrowUp' | 'Home' | 'End';

export function isNavigationKey(key: string): key is NavigationKey {
  return key === 'ArrowDown' || key === 'ArrowUp' || key === 'Home' || key === 'End';
}

/**
 * The entry to focus after `key`, from `current` among `count` entries:
 * the arrows wrap around, Home and End go to the first and last.
 */
export function step(current: number, key: NavigationKey, count: number): number {
  if (count === 0) return -1;
  switch (key) {
    case 'ArrowDown':
      return (current + 1) % count;
    case 'ArrowUp':
      return (current - 1 + count) % count;
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
  }
}

/** Where a popup of `width` × `height` at (x, y) fits into the window. */
export function fit(
  x: number,
  y: number,
  width: number,
  height: number,
  view: { width: number; height: number },
): { left: number; top: number } {
  const MARGIN = 4;
  return {
    left: Math.max(MARGIN, Math.min(x, view.width - width - MARGIN)),
    top: Math.max(MARGIN, Math.min(y, view.height - height - MARGIN)),
  };
}
