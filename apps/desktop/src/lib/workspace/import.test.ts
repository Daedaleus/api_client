import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import DialogHost from '../../components/organisms/DialogHost.svelte';
import { FakeApi, shop } from '../../test/fake-api';
import { i18n } from '../i18n/i18n.svelte';
import { Workspace } from './workspace.svelte';

function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  return { api, workspace };
}

const skipped = [{ key: 'import.skipped_scripts', args: { item: 'Login' } }];

describe('collection import', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('does nothing when the file or the folder is not chosen', async () => {
    const { api, workspace } = setup();
    await workspace.importCollection();
    api.file = 'C:\\exports\\shop.json';
    await workspace.importCollection();
    expect(api.commands).toEqual([]);
  });

  it('opens the imported collection and lists what was left out', async () => {
    const { api, workspace } = setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    api.file = 'C:\\exports\\shop.json';
    api.folder = 'C:\\apis\\shop';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped });
    const importing = workspace.importCollection();
    expect(await screen.findByRole('dialog')).toHaveTextContent('Import finished');
    expect(screen.getByText('“Login”: scripts (scripting comes later)')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'OK' }));
    await importing;
    expect(api.commands).toEqual([
      { type: 'importCollection', file: 'C:\\exports\\shop.json', path: 'C:\\apis\\shop' },
    ]);
    expect(workspace.collections.find('c1')?.name).toBe('Shop');
    expect(workspace.dialogs.current).toBeNull();
  });

  it('needs no dialog without left-out parts and reports errors', async () => {
    const { api, workspace } = setup();
    api.file = 'a.json';
    api.folder = 'b';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
    await workspace.importCollection();
    expect(workspace.dialogs.current).toBeNull();

    api.answers.push({ fail: { key: 'error.import_invalid', args: { detail: 'x' } } });
    await workspace.importCollection();
    expect(workspace.collections.error?.key).toBe('error.import_invalid');

    api.answers.push({ type: 'runStarted', runId: 1 });
    await workspace.importCollection();
    expect(workspace.collections.error?.args.detail).toBe('unexpected answer runStarted');
  });

  it('closes the note with Escape too', async () => {
    const { workspace } = setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    const done = vi.fn();
    void workspace.dialogs
      .inform({ key: 'import.done', args: {} }, { key: 'import.skipped_intro', args: {} }, skipped)
      .then(done);
    await screen.findByRole('dialog');
    await userEvent.keyboard('{Escape}');
    await vi.waitFor(() => expect(done).toHaveBeenCalled());
  });
});
