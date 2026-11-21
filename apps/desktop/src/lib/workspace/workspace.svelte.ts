// The workspace: open collections, editor tabs and the questions to the
// user, plus the actions that involve more than one of them (deleting a
// request also closes its tab, creating one opens it, …).

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { KeyValueStorage } from '../storage';
import type {
  CertificateCheckView,
  CollectionView,
  EnvironmentDraft,
  SendContext,
  TreeItemView,
  VariableDraft,
} from '../types';
import { scopeValues } from '../variables';
import { CollectionsState } from './collections.svelte';
import { Dialogs } from './dialogs.svelte';
import { EnvironmentChoice } from './environments.svelte';
import { RecipientsState } from './recipients.svelte';
import type { RequestTab } from './request-tab.svelte';
import { TabsState } from './tabs.svelte';
import { baseName, foldersAround } from './tree';
import { DefaultsState } from './defaults.svelte';

/** An environment open in the editor: a copy, so cancelling changes nothing. */
export interface EnvironmentEdit {
  collectionId: string;
  draft: EnvironmentDraft;
}

export class Workspace {
  readonly collections: CollectionsState;
  readonly tabs: TabsState;
  readonly dialogs = new Dialogs();
  readonly environments: EnvironmentChoice;
  /** Who secrets are encrypted for (dialog). */
  readonly recipients: RecipientsState;
  /** Variables and authentication of a collection or folder (dialog). */
  readonly defaults: DefaultsState;
  /** The environment being edited, if the editor is open. */
  editing = $state<EnvironmentEdit | null>(null);
  readonly #api: Api;

  constructor(api: Api, storage: KeyValueStorage | null) {
    this.#api = api;
    this.collections = new CollectionsState(api, storage);
    this.tabs = new TabsState(api);
    this.environments = new EnvironmentChoice(storage);
    this.recipients = new RecipientsState(api, this.collections);
    this.defaults = new DefaultsState(api, this.collections);
  }

  /** The collection a tab's request belongs to (none for scratch tabs). */
  collectionOf(tab: RequestTab): CollectionView | undefined {
    return tab.saved ? this.collections.find(tab.saved.collectionId) : undefined;
  }

  /** Listens for run results; resolves to an unsubscribe function. */
  connect(): Promise<() => void> {
    return this.#api.onEvent((event) => this.tabs.handle(event));
  }

  /** Opens the collections of the last session. */
  restore(): Promise<void> {
    return this.collections.restore();
  }

  /** Lets the user pick a file (for bodies); its path or `null`. */
  pickFile(): Promise<string | null> {
    return this.#api.pickFile();
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

  /** Sends a tab's request in its collection with the chosen environment. */
  send(tab: RequestTab): Promise<void> {
    return this.tabs.send(tab, this.#context(tab));
  }

  /**
   * The values a tab's `{{variables}}` get, for the preview: collection,
   * environment, folders and the request itself, the stronger winning.
   */
  variablesFor(tab: RequestTab): Map<string, string> {
    const layers: VariableDraft[][] = [];
    const collection = this.collectionOf(tab);
    if (collection && tab.saved) {
      layers.push(collection.variables);
      const environment = this.environments.of(collection);
      if (environment) layers.push(environment.variables);
      for (const folder of foldersAround(collection, tab.saved.requestId)) {
        layers.push(folder.variables);
      }
    }
    layers.push(tab.draft.variables);
    return scopeValues(layers);
  }

  /**
   * Whether the server certificate is checked for a tab's request: its own
   * setting, else the innermost folder or the collection that has one; on
   * by default (as when sending, ac_domain::tls).
   */
  checksCertificates(tab: RequestTab): boolean {
    const chain: CertificateCheckView[] = [tab.draft.certificateCheck];
    const collection = this.collectionOf(tab);
    if (collection && tab.saved) {
      const folders = foldersAround(collection, tab.saved.requestId).reverse();
      chain.push(...folders.map((folder) => folder.certificateCheck), collection.certificateCheck);
    }
    return (chain.find((check) => check !== 'inherit') ?? 'on') === 'on';
  }

  #context(tab: RequestTab): SendContext | null {
    const collection = this.collectionOf(tab);
    if (!collection || !tab.saved) return null;
    return {
      collectionId: collection.id,
      requestId: tab.saved.requestId,
      environmentId: this.environments.of(collection)?.id ?? null,
    };
  }

  async newEnvironment(collectionId: string): Promise<void> {
    const name = await this.dialogs.askName({ key: 'dialog.new_environment', args: {} });
    if (!name) return;
    const changed = await this.collections.run({ type: 'createEnvironment', collectionId, name });
    if (!changed?.createdId) return;
    this.environments.choose(collectionId, changed.createdId);
    this.editEnvironment(collectionId, changed.createdId);
  }

  /** Opens the editor with a copy of an environment. */
  editEnvironment(collectionId: string, environmentId: string): void {
    const environment = this.collections
      .find(collectionId)
      ?.environments.find((candidate) => candidate.id === environmentId);
    if (environment) {
      this.collections.error = null;
      this.editing = { collectionId, draft: $state.snapshot(environment) };
    }
  }

  /** Saves the edited environment; the editor stays open if that fails. */
  async saveEnvironment(): Promise<void> {
    if (!this.editing) return;
    const { collectionId, draft } = this.editing;
    const changed = await this.collections.run({
      type: 'saveEnvironment',
      collectionId,
      environment: $state.snapshot(draft),
    });
    if (changed) this.editing = null;
  }

  async deleteEnvironment(): Promise<void> {
    if (!this.editing) return;
    const { collectionId, draft } = this.editing;
    const ok = await this.dialogs.confirm(
      { key: 'dialog.delete', args: {} },
      { key: 'dialog.delete_environment', args: { name: draft.name } },
    );
    if (!ok) return;
    const changed = await this.collections.run({
      type: 'deleteItem',
      collectionId,
      itemId: draft.id,
    });
    if (!changed) return;
    this.editing = null;
    if (this.environments.of(changed.collection) === null) {
      this.environments.choose(collectionId, null);
    }
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
