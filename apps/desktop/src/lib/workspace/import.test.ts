import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import DialogHost from '../../components/organisms/DialogHost.svelte';
import { FakeApi, memoryStorage, shop } from '../../test/fake-api';
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

  it('does nothing when no file is chosen', async () => {
    const { api, workspace } = setup();
    await workspace.importCollection('postman');
    expect(api.commands).toEqual([]);
  });

  it('opens the imported collection and lists what was left out', async () => {
    const { api, workspace } = setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    api.file = 'C:\\exports\\shop.json';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped });
    const importing = workspace.importCollection('postman');
    expect(await screen.findByRole('dialog')).toHaveTextContent('Import finished');
    expect(screen.getByText('“Login”: scripts (not taken over yet)')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'OK' }));
    await importing;
    expect(api.commands).toEqual([
      { type: 'importCollection', file: 'C:\\exports\\shop.json', path: null },
    ]);
    expect(workspace.collections.find('c1')?.name).toBe('Shop');
    expect(workspace.dialogs.current).toBeNull();
  });

  it('needs no dialog without left-out parts and reports errors', async () => {
    const { api, workspace } = setup();
    api.file = 'a.json';
    api.folder = 'b';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
    await workspace.importCollection('postman');
    expect(workspace.dialogs.current).toBeNull();

    api.answers.push({ fail: { key: 'error.import_invalid', args: { detail: 'x' } } });
    await workspace.importCollection('postman');
    expect(workspace.collections.error?.key).toBe('error.import_invalid');

    api.answers.push({ type: 'runStarted', runId: 1 });
    await workspace.importCollection('postman');
    expect(workspace.collections.error?.args.detail).toBe('unexpected answer runStarted');
  });

  it('offers a file filter per format and asks for no target folder', async () => {
    const { api, workspace } = setup();
    api.file = 'C:\\exports\\shop.yaml';
    api.folder = 'C:\\apis\\shop';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
    await workspace.importCollection('openapi');
    expect(api.picks).toEqual([
      {
        kind: 'file',
        options: {
          title: 'Import an OpenAPI description',
          filters: [
            { name: 'OpenAPI (JSON, YAML)', extensions: ['json', 'yaml', 'yml'] },
            { name: 'All files', extensions: ['*'] },
          ],
        },
      },
    ]);
  });

  it('saves an unsaved collection where the user chooses and forgets it unsaved', async () => {
    const storage = memoryStorage();
    const api = new FakeApi();
    const workspace = new Workspace(api, storage);
    api.file = 'C:\\exports\\shop.json';
    const unsaved = { ...shop(), temporary: true, root: 'C:\\Temp\\api-client-unsaved\\x' };
    api.answers.push({ type: 'collectionImported', collection: unsaved, skipped: [] });
    await workspace.importCollection('postman');
    expect(storage.data.get('openCollections')).toBe('[]');

    await workspace.saveCollectionAs('c1');
    expect(api.commands).toHaveLength(1);
    api.folder = 'C:\\apis\\shop';
    api.answers.push({ type: 'collectionChanged', collection: shop(), createdId: null });
    await workspace.saveCollectionAs('c1');
    expect(api.commands.at(-1)).toEqual({
      type: 'saveCollectionAs',
      collectionId: 'c1',
      path: 'C:\\apis\\shop',
    });
    expect(api.picks.at(-1)).toEqual({
      kind: 'folder',
      options: { title: 'Choose the folder for the new collection' },
    });
    expect(workspace.collections.find('c1')?.temporary).toBe(false);
    expect(storage.data.get('openCollections')).toBe(JSON.stringify(['C:\\apis\\shop']));
  });

  it('asks before closing an unsaved collection', async () => {
    const { api, workspace } = setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    workspace.collections.apply({ ...shop(), temporary: true });
    const closing = workspace.closeCollection('c1');
    expect(await screen.findByRole('dialog')).toHaveTextContent('Discard collection?');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await closing;
    expect(workspace.collections.find('c1')).toBeDefined();

    const confirmed = workspace.closeCollection('c1');
    await screen.findByRole('dialog');
    await userEvent.click(screen.getByRole('button', { name: 'OK' }));
    await confirmed;
    expect(api.commands.at(-1)).toEqual({ type: 'closeCollection', collectionId: 'c1' });
    expect(workspace.collections.find('c1')).toBeUndefined();
  });

  it('imports a Bruno collection from its folder', async () => {
    const { api, workspace } = setup();
    api.folder = 'C:\\apis\\bruno-shop\\';
    api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
    await workspace.importCollection('bruno');
    expect(api.picks[0]).toEqual({
      kind: 'folder',
      options: { title: 'Choose the folder of the Bruno collection', filters: [] },
    });
    expect(api.commands[0]).toMatchObject({
      file: 'C:\\apis\\bruno-shop\\bruno.json',
    });
  });

  it('imports from a URL the user enters', async () => {
    const { api, workspace } = setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    const cancelled = workspace.importFromUrl();
    expect(await screen.findByRole('dialog')).toHaveTextContent('Import from a URL');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await cancelled;
    expect(api.importedUrls).toEqual([]);

    api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
    const importing = workspace.importFromUrl();
    const field = await screen.findByLabelText(
      'Address of the OpenAPI description or Postman collection',
    );
    await userEvent.type(field, 'https://example.com/openapi.json{Enter}');
    await importing;
    expect(api.importedUrls).toEqual(['https://example.com/openapi.json']);
    expect(workspace.collections.find('c1')?.name).toBe('Shop');

    api.answers.push({ fail: { key: 'error.import_download', args: { detail: 'status 404' } } });
    const failing = workspace.importFromUrl();
    await userEvent.type(
      await screen.findByLabelText('Address of the OpenAPI description or Postman collection'),
      'https://example.com/gone{Enter}',
    );
    await failing;
    expect(workspace.collections.error?.key).toBe('error.import_download');
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
