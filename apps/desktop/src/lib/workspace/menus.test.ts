import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../i18n/i18n.svelte';
import type { MenuEntry, MenuItem } from '../menu';
import { shop } from '../../test/fake-api';
import type { TreeItemView } from '../types';
import { collectionMenu, newMenu, treeItemMenu } from './menus';
import type { Workspace } from './workspace.svelte';

/** A workspace whose actions are spies. */
function workspace() {
  return {
    tabs: { openScratch: vi.fn() },
    curl: { openImport: vi.fn() },
    defaults: { open: vi.fn() },
    createCollection: vi.fn(),
    importCollection: vi.fn(),
    importFromUrl: vi.fn(),
    newRequest: vi.fn(),
    newFolder: vi.fn(),
    rename: vi.fn(),
    duplicate: vi.fn(),
    moveBy: vi.fn(),
    moveTo: vi.fn(),
    remove: vi.fn(),
    closeCollection: vi.fn(),
    saveCollectionAs: vi.fn(),
    exportCollection: vi.fn(),
  };
}

const entries = (items: MenuItem[]) =>
  items.filter((item): item is MenuEntry => item.kind === 'entry');

/** Chooses every entry (also in submenus) and returns their ids. */
function chooseAll(items: MenuItem[]): string[] {
  return entries(items).flatMap((item) => {
    item.onSelect?.();
    return [item.id, ...(item.items ? chooseAll(item.items) : [])];
  });
}

describe('sidebar menus', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('"New" creates and imports', () => {
    const fake = workspace();
    const items = newMenu(fake as unknown as Workspace);
    expect(entries(items)[0]).toMatchObject({ label: 'New request', shortcut: 'Ctrl+N' });
    expect(chooseAll(items)).toEqual([
      'new-request',
      'new-collection',
      'import',
      'import-postman',
      'import-openapi',
      'import-bruno',
      'import-url',
      'import-curl',
    ]);
    expect(fake.tabs.openScratch).toHaveBeenCalledOnce();
    expect(fake.createCollection).toHaveBeenCalledOnce();
    expect(fake.importCollection.mock.calls).toEqual([['postman'], ['openapi'], ['bruno']]);
    expect(fake.curl.openImport).toHaveBeenCalledOnce();
  });

  it('acts on a collection', () => {
    const fake = workspace();
    const collection = shop();
    expect(chooseAll(collectionMenu(fake as unknown as Workspace, collection))).toEqual([
      'new-request',
      'new-folder',
      'settings',
      'export-postman',
      'close',
    ]);
    expect(fake.newRequest).toHaveBeenCalledWith('c1', 'c1');
    expect(fake.newFolder).toHaveBeenCalledWith('c1', 'c1');
    expect(fake.defaults.open).toHaveBeenCalledWith('c1', 'c1');
    expect(fake.exportCollection).toHaveBeenCalledWith('c1');
    expect(fake.closeCollection).toHaveBeenCalledWith('c1');
  });

  it('offers to save an unsaved collection first', () => {
    const fake = workspace();
    const items = collectionMenu(fake as unknown as Workspace, { ...shop(), temporary: true });
    expect(chooseAll(items)[0]).toBe('save-as');
    expect(fake.saveCollectionAs).toHaveBeenCalledWith('c1');
  });

  it('acts on folders and requests in the tree', () => {
    const fake = workspace();
    const [folder, request] = shop().items as [TreeItemView, TreeItemView];
    const ws = fake as unknown as Workspace;
    expect(chooseAll(treeItemMenu(ws, 'c1', folder))).toEqual([
      'new-request',
      'new-folder',
      'settings',
      'rename',
      'duplicate',
      'move',
      'move-up',
      'move-down',
      'move-to',
      'delete',
    ]);
    expect(fake.newRequest).toHaveBeenCalledWith('c1', folder.id);
    expect(fake.defaults.open).toHaveBeenCalledWith('c1', folder.id);
    expect(chooseAll(treeItemMenu(ws, 'c1', request))).toEqual([
      'rename',
      'duplicate',
      'move',
      'move-up',
      'move-down',
      'move-to',
      'delete',
    ]);
    expect(fake.rename).toHaveBeenCalledWith('c1', request);
    expect(fake.duplicate).toHaveBeenCalledWith('c1', request);
    expect(fake.moveBy).toHaveBeenCalledWith('c1', request, -1);
    expect(fake.moveBy).toHaveBeenCalledWith('c1', request, 1);
    expect(fake.moveTo).toHaveBeenCalledWith('c1', request);
    expect(fake.remove).toHaveBeenCalledWith('c1', request);
  });
});
