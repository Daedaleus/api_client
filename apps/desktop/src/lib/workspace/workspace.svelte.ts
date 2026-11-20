// The workspace: open collections, editor tabs and the questions to the
// user, plus the actions that involve more than one of them (deleting a
// request also closes its tab, creating one opens it, …).

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { TreeItemView } from '../types';
import { CollectionsState } from './collections.svelte';
import { Dialogs } from './dialogs.svelte';
import type { RequestTab } from './request-tab.svelte';
import { TabsState } from './tabs.svelte';
import { baseName } from './tree';

type PathStorage = Pick<Storage, 'getItem' | 'setItem'>;

export class Workspace {
  readonly collections: CollectionsState;
  readonly tabs: TabsState;
  readonly dialogs = new Dialogs();
  readonly #api: Api;

  constructor(api: Api, storage: PathStorage | null) {
    this.#api = api;
    this.collections = new CollectionsState(api, storage);
    this.tabs = new TabsState(api);
  }

  /** Listens for run results; resolves to an unsubscribe function. */
  connect(): Promise<() => void> {
    return this.#api.onEvent((event) => this.tabs.handle(event));
  }

  /** Opens the collections of the last session. */
  restore(): Promise<void> {
    return this.collections.restore();
  }

  async openCollection(): Promise<void> {
    const path = await this.#api.pickFolder();
    if (path) await this.collections.run({ type: 'openCollection', path });
  }

  async createCollection(): Promise<void> {
    const path = await this.#api.pickFolder();
    if (!path) return;
    const name = await this.dialogs.askName(
      { key: 'dialog.new_collection', args: {} },
      baseName(path),
    );
    if (name) await this.collections.run({ type: 'createCollection', path, name });
  }

  async closeCollection(collectionId: string): Promise<void> {
    const dirty = this.tabs.tabs.some((tab) => tab.isFor(collectionId) && tab.dirty);
    if (dirty && !(await this.#confirmDiscard())) return;
    this.tabs.closeCollection(collectionId);
    await this.collections.close(collectionId);
  }

  async openRequest(collectionId: string, requestId: string): Promise<void> {
    this.collections.error = null;
    try {
      await this.tabs.openSaved(collectionId, requestId);
    } catch (error) {
      this.collections.error = toUserText(error);
    }
  }

  async newRequest(collectionId: string, parentId: string): Promise<void> {
    const name = await this.dialogs.askName({ key: 'dialog.new_request', args: {} });
    if (!name) return;
    const changed = await this.collections.run({
      type: 'createRequest',
      collectionId,
      parentId,
      name,
    });
    if (changed?.createdId) await this.openRequest(collectionId, changed.createdId);
  }

  async newFolder(collectionId: string, parentId: string): Promise<void> {
    const name = await this.dialogs.askName({ key: 'dialog.new_folder', args: {} });
    if (name) await this.collections.run({ type: 'createFolder', collectionId, parentId, name });
  }

  async rename(collectionId: string, item: TreeItemView): Promise<void> {
    const name = await this.dialogs.askName({ key: 'dialog.rename', args: {} }, item.name);
    if (!name || name === item.name) return;
    const changed = await this.collections.run({
      type: 'renameItem',
      collectionId,
      itemId: item.id,
      name,
    });
    if (changed && item.kind === 'request') this.tabs.renamed(collectionId, item.id, name);
  }

  async remove(collectionId: string, item: TreeItemView): Promise<void> {
    const ok = await this.dialogs.confirm(
      { key: 'dialog.delete', args: {} },
      { key: `dialog.delete_${item.kind}`, args: { name: item.name } },
    );
    if (!ok) return;
    const changed = await this.collections.run({
      type: 'deleteItem',
      collectionId,
      itemId: item.id,
    });
    if (changed) this.tabs.closeMissing(changed.collection);
  }

  async save(tab: RequestTab): Promise<void> {
    const collection = await this.tabs.save(tab);
    if (collection) this.collections.apply(collection);
  }

  async closeTab(tab: RequestTab): Promise<void> {
    if (tab.dirty && !(await this.#confirmDiscard())) return;
    this.tabs.close(tab.key);
  }

  #confirmDiscard(): Promise<boolean> {
    return this.dialogs.confirm(
      { key: 'dialog.discard', args: {} },
      { key: 'dialog.discard_message', args: {} },
    );
  }
}
