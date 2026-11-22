// The workspace: open collections, editor tabs and the questions to the
// user, plus the actions that involve more than one of them (deleting a
// request also closes its tab, creating one opens it, …).

import type { Api } from '../api';
import { toUserText } from '../errors';
import { i18n } from '../i18n/i18n.svelte';
import { readJson, writeJson, type KeyValueStorage } from '../storage';
import type {
  CertificateCheckView,
  CollectionView,
  CommandAck,
  EnvironmentDraft,
  SendContext,
  TreeItemView,
  VariableDraft,
} from '../types';
import { apiAssist, type ScriptAssist } from '../script-assist';
import { scopeInfo, scopeValues, type VariableInfo, type VariableSource } from '../variables';
import { CollectionsState } from './collections.svelte';
import { Dialogs } from './dialogs.svelte';
import { EnvironmentChoice } from './environments.svelte';
import {
  type ImportFormat,
  isFolder,
  sourceFile,
  sourceOptions,
  targetOptions,
} from './import-source';
import { RecipientsState } from './recipients.svelte';
import type { RequestTab } from './request-tab.svelte';
import { TabsState } from './tabs.svelte';
import { baseName, dropPlace, foldersAround, locate, moveTargets } from './tree';
import { TreeDrag } from './drag.svelte';
import { isSession, sessionOf } from './session';
import { HistoryState } from './history.svelte';
import { CurlState } from './curl.svelte';
import { DefaultsState } from './defaults.svelte';

/** An environment open in the editor: a copy, so cancelling changes nothing. */
export interface EnvironmentEdit {
  collectionId: string;
  draft: EnvironmentDraft;
}

/** Storage key of the open tabs (see `session.ts`). */
const SESSION_KEY = 'openTabs';

export class Workspace {
  readonly collections: CollectionsState;
  readonly tabs: TabsState;
  readonly dialogs = new Dialogs();
  readonly environments: EnvironmentChoice;
  /** Who secrets are encrypted for (dialog). */
  readonly recipients: RecipientsState;
  /** Variables and authentication of a collection or folder (dialog). */
  readonly defaults: DefaultsState;
  /** cURL export and import. */
  readonly curl: CurlState;
  /** Checker and completion of the script editor. */
  readonly scriptAssist: ScriptAssist;
  /** Dragging items in the sidebar tree. */
  readonly drag = new TreeDrag();
  /** Requests sent before (sidebar). */
  readonly history: HistoryState;
  /** The quick search (Ctrl+K) is open. */
  searching = $state(false);
  /** The filter above the sidebar tree. */
  treeFilter = $state('');
  /** The environment being edited, if the editor is open. */
  editing = $state<EnvironmentEdit | null>(null);
  readonly #api: Api;
  readonly #storage: KeyValueStorage | null;
  /** The last session's tabs are back: from now on the open tabs are remembered. */
  #tabsRestored = false;

  constructor(api: Api, storage: KeyValueStorage | null) {
    this.scriptAssist = apiAssist(api);
    this.#api = api;
    this.#storage = storage;
    this.collections = new CollectionsState(api, storage);
    this.tabs = new TabsState(api);
    this.environments = new EnvironmentChoice(storage);
    this.recipients = new RecipientsState(api, this.collections);
    this.defaults = new DefaultsState(api, this.collections);
    this.curl = new CurlState(api, this.tabs);
    this.history = new HistoryState(api, this.tabs, this.dialogs);
  }

  /** The collection a tab's request belongs to (none for scratch tabs). */
  collectionOf(tab: RequestTab): CollectionView | undefined {
    return tab.saved ? this.collections.find(tab.saved.collectionId) : undefined;
  }

  /** Listens for run results and history changes; resolves to an unsubscribe function. */
  connect(): Promise<() => void> {
    return this.#api.onEvent((event) => {
      if (event.type === 'historyChanged') void this.history.changed(event.error);
      else this.tabs.handle(event);
    });
  }

  /** Opens the collections and then the tabs of the last session. */
  async restore(): Promise<void> {
    await this.collections.restore();
    const session = readJson(this.#storage, SESSION_KEY, null, isSession);
    if (session) {
      await this.tabs.restore(session, (id) => this.collections.find(id) !== undefined);
    }
    this.#tabsRestored = true;
  }

  /** Remembers the open tabs for the next start (once the last ones are back). */
  rememberTabs(): void {
    if (!this.#tabsRestored) return;
    writeJson(this.#storage, SESSION_KEY, sessionOf(this.tabs.tabs, this.tabs.activeKey));
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

  /**
   * Imports another tool's collection: the user picks the file (for Bruno
   * the folder). The collection stays unsaved – in a temporary folder –
   * until "save as". What could not be taken over is shown afterwards.
   */
  async importCollection(format: ImportFormat): Promise<void> {
    const options = sourceOptions(format);
    const picked = isFolder(format)
      ? await this.#api.pickFolder(options)
      : await this.#api.pickFile(options);
    if (!picked) return;
    const file = sourceFile(format, picked);
    this.collections.error = null;
    try {
      await this.#finishImport(
        await this.#api.execute({ type: 'importCollection', file, path: null }),
      );
    } catch (error) {
      this.collections.error = toUserText(error);
    }
  }

  /**
   * Downloads a collection or description from a URL the user enters and
   * opens it unsaved; relative servers are resolved against the URL.
   */
  async importFromUrl(): Promise<void> {
    const url = await this.dialogs.askText(
      { key: 'import.title_url', args: {} },
      { key: 'import.url_label', args: {} },
    );
    if (!url) return;
    this.collections.error = null;
    try {
      await this.#finishImport(await this.#api.importUrl(url));
    } catch (error) {
      this.collections.error = toUserText(error);
    }
  }

  /** Opens an imported collection and lists what was left out. */
  async #finishImport(ack: CommandAck): Promise<void> {
    if (ack.type !== 'collectionImported') throw new Error(`unexpected answer ${ack.type}`);
    this.collections.apply(ack.collection);
    if (ack.skipped.length > 0) {
      await this.dialogs.inform(
        { key: 'import.done', args: {} },
        { key: 'import.skipped_intro', args: {} },
        ack.skipped,
      );
    }
  }

  /** Saves an unsaved collection where the user chooses. */
  async saveCollectionAs(collectionId: string): Promise<void> {
    const path = await this.#api.pickFolder(targetOptions());
    if (!path) return;
    await this.collections.run({ type: 'saveCollectionAs', collectionId, path });
  }

  async closeCollection(collectionId: string): Promise<void> {
    const dirty = this.tabs.tabs.some((tab) => tab.isFor(collectionId) && tab.dirty);
    if (dirty && !(await this.#confirmDiscard())) return;
    const unsaved = this.collections.find(collectionId)?.temporary === true;
    if (
      unsaved &&
      !(await this.dialogs.confirm(
        { key: 'dialog.discard_collection', args: {} },
        { key: 'dialog.discard_collection_message', args: {} },
      ))
    ) {
      return;
    }
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

  /** Copies a request or folder (with everything in it) right after it. */
  async duplicate(collectionId: string, item: TreeItemView): Promise<void> {
    await this.collections.run({
      type: 'duplicateItem',
      collectionId,
      itemId: item.id,
      name: i18n.t('collection.copy_name', { name: item.name }),
    });
  }

  /** Moves an item into the collection or folder `parentId` at `index`. */
  async move(collectionId: string, itemId: string, parentId: string, index: number): Promise<void> {
    await this.collections.run({ type: 'moveItem', collectionId, itemId, parentId, index });
  }

  /** Moves an item one place up (`-1`) or down (`1`) within its folder. */
  async moveBy(collectionId: string, item: TreeItemView, step: -1 | 1): Promise<void> {
    const collection = this.collections.find(collectionId);
    const place = collection && locate(collection, item.id);
    if (!place) return;
    const index = place.index + step;
    if (index < 0 || index >= place.count) return;
    await this.move(collectionId, item.id, place.parentId, index);
  }

  /** Asks for a folder (or the collection) and moves the item to its end. */
  async moveTo(collectionId: string, item: TreeItemView): Promise<void> {
    const collection = this.collections.find(collectionId);
    if (!collection) return;
    const targets = moveTargets(collection, item.id);
    const chosen = await this.dialogs.choose(
      { key: 'collection.move_to_title', args: { name: item.name } },
      { key: 'collection.move_to_label', args: {} },
      targets.map((target) => ({ value: target.id, label: target.label })),
    );
    const target = targets.find((candidate) => candidate.id === chosen);
    if (!target) return;
    await this.move(collectionId, item.id, target.id, target.count);
  }

  /** Ends a drag in the tree: moves the item where it was dropped. */
  async drop(): Promise<void> {
    const dropped = this.drag.release();
    if (!dropped) return;
    const collection = this.collections.find(dropped.source.collectionId);
    if (!collection) return;
    const place = dropPlace(
      collection,
      dropped.source.item.id,
      dropped.target.item,
      dropped.target.position,
    );
    if (!place) return;
    await this.move(collection.id, dropped.source.item.id, place.parentId, place.index);
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

  /** A tab's request as a cURL command, exactly as it would be sent. */
  curlFor(tab: RequestTab): Promise<string | null> {
    return this.curl.command(tab, this.tabs.outgoing(tab, this.#context(tab)));
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

  /** The variables of a tab's request with where each comes from (see `variablesFor`). */
  variableInfo(tab: RequestTab): Map<string, VariableInfo> {
    const layers: { source: VariableSource; variables: VariableDraft[] }[] = [];
    const collection = this.collectionOf(tab);
    if (collection && tab.saved) {
      layers.push({ source: 'collection', variables: collection.variables });
      const environment = this.environments.of(collection);
      if (environment) layers.push({ source: 'environment', variables: environment.variables });
      for (const folder of foldersAround(collection, tab.saved.requestId)) {
        layers.push({ source: 'folder', variables: folder.variables });
      }
    }
    layers.push({ source: 'request', variables: tab.draft.variables });
    return scopeInfo(layers);
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
