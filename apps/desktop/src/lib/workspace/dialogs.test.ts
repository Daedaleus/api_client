import { describe, expect, it } from 'vitest';

import { Dialogs } from './dialogs.svelte';

const TITLE = { key: 'dialog.rename', args: {} };

describe('Dialogs', () => {
  it('resolves names trimmed, and empty or cancelled as null', async () => {
    const dialogs = new Dialogs();
    const asked = dialogs.askName(TITLE, 'Old');
    expect(dialogs.current).toMatchObject({ kind: 'name', value: 'Old' });
    if (dialogs.current?.kind === 'name') dialogs.current.answer('  New  ');
    expect(await asked).toBe('New');
    expect(dialogs.current).toBeNull();

    const empty = dialogs.askName(TITLE);
    if (dialogs.current?.kind === 'name') dialogs.current.answer('  ');
    expect(await empty).toBeNull();

    const cancelled = dialogs.askName(TITLE);
    if (dialogs.current?.kind === 'name') dialogs.current.answer(null);
    expect(await cancelled).toBeNull();
  });

  it('resolves confirmations', async () => {
    const dialogs = new Dialogs();
    const asked = dialogs.confirm(TITLE, { key: 'dialog.discard_message', args: {} });
    if (dialogs.current?.kind === 'confirm') dialogs.current.answer(true);
    expect(await asked).toBe(true);
    expect(dialogs.current).toBeNull();
  });
});
