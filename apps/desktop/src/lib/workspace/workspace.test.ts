import { describe, expect, it, vi } from 'vitest';

import { FakeApi, OK, changed, memoryStorage, shop } from '../../test/fake-api';
import type { TreeItemView } from '../types';
import { Workspace } from './workspace.svelte';

/** A workspace with `Shop` open. */
async function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, memoryStorage());
  workspace.collections.apply(shop());
  return { api, workspace };
}

/** Waits until the action under test opens its dialog. */
function openDialog(workspace: Workspace) {
  return vi.waitFor(() => {
    const dialog = workspace.dialogs.current;
    if (!dialog) throw new Error('no dialog open yet');
    return dialog;
  });
}

/** Answers the dialog as soon as the action opens it. */
async function answer(workspace: Workspace, value: string | boolean | null) {
  const dialog = await openDialog(workspace);
  if (dialog.kind === 'name') dialog.answer(value as string | null);
  else dialog.answer(value as boolean);
}

const health: TreeItemView = shop().items[1]!;

describe('Workspace', () => {
  it('routes run results from the backend to the tabs', async () => {
    const { api, workspace } = await setup();
    const stop = await workspace.connect();
    await workspace.tabs.send(workspace.tabs.active!);
    api.emit(1, OK);
    expect(workspace.tabs.active?.outcome).toEqual(OK);
    stop();
    expect(api.unsubscribed).toBe(true);
  });

  it('opens and creates collections in picked folders', async () => {
    const { api, workspace } = await setup();
    await workspace.openCollection();
    expect(api.commands).toHaveLength(0);

    api.folder = 'C:\\apis\\billing';
    api.answers.push(changed({ ...shop(), id: 'c2', name: 'Billing' }));
    await workspace.openCollection();
    expect(api.commands.at(-1)).toEqual({ type: 'openCollection', path: 'C:\\apis\\billing' });

    const creating = workspace.createCollection();
    await openDialog(workspace);
    expect(workspace.dialogs.current).toMatchObject({ kind: 'name', value: 'billing' });
    api.answers.push(changed({ ...shop(), id: 'c3', name: 'Payments' }));
    await answer(workspace, 'Payments');
    await creating;
    expect(api.commands.at(-1)).toEqual({
      type: 'createCollection',
      path: 'C:\\apis\\billing',
      name: 'Payments',
    });
    expect(workspace.collections.collections).toHaveLength(3);
  });

  it('cancelling a dialog changes nothing', async () => {
    const { api, workspace } = await setup();
    api.folder = 'C:\\x';
    const creating = workspace.createCollection();
    await answer(workspace, null);
    await creating;

    api.folder = null;
    await workspace.createCollection();

    const naming = workspace.newRequest('c1', 'c1');
    await answer(workspace, '   ');
    await naming;
    expect(api.commands).toHaveLength(0);
  });

  it('creates requests and opens them', async () => {
    const { api, workspace } = await setup();
    api.answers.push(changed(shop(), 'r7'));
    const creating = workspace.newRequest('c1', 'f1');
    await answer(workspace, 'Logout');
    await creating;
    expect(api.commands[0]).toEqual({
      type: 'createRequest',
      collectionId: 'c1',
      parentId: 'f1',
      name: 'Logout',
    });
    expect(workspace.tabs.active?.saved).toEqual({ collectionId: 'c1', requestId: 'r7' });
  });

  it('creates folders', async () => {
    const { api, workspace } = await setup();
    api.answers.push(changed(shop(), 'f9'));
    const creating = workspace.newFolder('c1', 'c1');
    await answer(workspace, 'Orders');
    await creating;
    expect(api.commands[0]).toMatchObject({ type: 'createFolder', name: 'Orders' });
  });

  it('renames items and open tabs follow', async () => {
    const { api, workspace } = await setup();
    await workspace.openRequest('c1', 'r2');
    api.answers.push(changed(shop()));
    const renaming = workspace.rename('c1', health);
    await openDialog(workspace);
    expect(workspace.dialogs.current).toMatchObject({ value: 'Health' });
    await answer(workspace, 'Ping');
    await renaming;
    expect(api.commands[0]).toEqual({
      type: 'renameItem',
      collectionId: 'c1',
      itemId: 'r2',
      name: 'Ping',
    });
    expect(workspace.tabs.active?.draft.name).toBe('Ping');

    // The same name changes nothing.
    const same = workspace.rename('c1', health);
    await answer(workspace, 'Health');
    await same;
    expect(api.commands).toHaveLength(1);
  });

  it('deletes after confirmation and closes the tabs of deleted requests', async () => {
    const { api, workspace } = await setup();
    await workspace.openRequest('c1', 'r2');
    const declined = workspace.remove('c1', health);
    await openDialog(workspace);
    expect(workspace.dialogs.current).toMatchObject({
      kind: 'confirm',
      message: { key: 'dialog.delete_request', args: { name: 'Health' } },
    });
    await answer(workspace, false);
    await declined;
    expect(api.commands).toHaveLength(0);

    const withoutHealth = { ...shop(), items: shop().items.slice(0, 1) };
    api.answers.push(changed(withoutHealth));
    const deleting = workspace.remove('c1', health);
    await answer(workspace, true);
    await deleting;
    expect(api.commands[0]).toEqual({ type: 'deleteItem', collectionId: 'c1', itemId: 'r2' });
    expect(workspace.tabs.tabs.some((tab) => tab.isFor('c1', 'r2'))).toBe(false);
  });

  it('asks before closing tabs or collections with unsaved changes', async () => {
    const { api, workspace } = await setup();
    await workspace.openRequest('c1', 'r2');
    const tab = workspace.tabs.active!;
    tab.draft.url = 'changed';

    const keep = workspace.closeTab(tab);
    await answer(workspace, false);
    await keep;
    expect(workspace.tabs.tabs).toContain(tab);

    const keepCollection = workspace.closeCollection('c1');
    await answer(workspace, false);
    await keepCollection;
    expect(workspace.collections.collections).toHaveLength(1);

    api.answers.push({ type: 'collectionClosed', collectionId: 'c1' });
    const closing = workspace.closeCollection('c1');
    await answer(workspace, true);
    await closing;
    expect(workspace.collections.collections).toHaveLength(0);
    expect(workspace.tabs.tabs).not.toContain(tab);
  });

  it('closes clean tabs without asking', async () => {
    const { workspace } = await setup();
    await workspace.openRequest('c1', 'r2');
    await workspace.closeTab(workspace.tabs.active!);
    expect(workspace.dialogs.current).toBeNull();
    expect(workspace.tabs.tabs).toHaveLength(1);
  });

  it('saves and takes over the collection the backend reports', async () => {
    const { api, workspace } = await setup();
    await workspace.openRequest('c1', 'r2');
    api.answers.push(changed({ ...shop(), name: 'Shop (saved)' }));
    await workspace.save(workspace.tabs.active!);
    expect(workspace.collections.find('c1')?.name).toBe('Shop (saved)');

    api.answers.push({ fail: { key: 'error.store_io', args: {} } });
    await workspace.save(workspace.tabs.active!);
    expect(workspace.collections.find('c1')?.name).toBe('Shop (saved)');
  });

  it('shows why a request could not be opened', async () => {
    const { api, workspace } = await setup();
    api.queryAnswers.push({ fail: { key: 'error.unknown_item', args: {} } });
    await workspace.openRequest('c1', 'gone');
    expect(workspace.collections.error?.key).toBe('error.unknown_item');
  });

  it('restores the collections of the last session', async () => {
    const api = new FakeApi();
    const storage = memoryStorage({ openCollections: JSON.stringify(['C:\\apis\\shop']) });
    const workspace = new Workspace(api, storage);
    api.answers.push(changed(shop()));
    await workspace.restore();
    expect(workspace.collections.collections).toHaveLength(1);
  });
});
