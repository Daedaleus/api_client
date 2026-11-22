// Keyboard shortcuts of the workspace (CLAUDE.md: full keyboard operation).
// Ctrl on Windows and Linux, Cmd on macOS.

/** What a shortcut does. */
export type ShortcutAction = 'send' | 'save' | 'newRequest' | 'environment' | 'search';

type KeyEvent = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>;

/** The action of a key press, or `null` for any other key. */
export function shortcutFor(event: KeyEvent): ShortcutAction | null {
  const command = event.ctrlKey || event.metaKey;
  if (!command || event.altKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'enter') return 'send';
  if (event.shiftKey) return null;
  switch (key) {
    case 's':
      return 'save';
    case 'n':
      return 'newRequest';
    case 'e':
      return 'environment';
    case 'k':
      return 'search';
    default:
      return null;
  }
}
