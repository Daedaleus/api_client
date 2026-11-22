// The open editor tabs: opening saved requests, sending, saving, and routing
// run results to the tab that started the run.

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { AppEvent, CollectionView, OutgoingRequest, RunOutcome, SendContext } from '../types';
import { RequestTab } from './request-tab.svelte';
import { requestIds } from './tree';

export class TabsState {
  tabs = $state<RequestTab[]>([]);
  activeKey = $state<string | null>(null);

  readonly #api: Api;
  /** Results that arrived before their acknowledgement; never rendered, so not reactive. */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  readonly #early = new Map<number, RunOutcome>();

  constructor(api: Api) {
    this.#api = api;
    this.openScratch();
  }

  get active(): RequestTab | null {
    return this.tabs.find((tab) => tab.key === this.activeKey) ?? null;
  }

  activate(key: string): void {
    this.activeKey = key;
  }

  openScratch(): RequestTab {
    const tab = RequestTab.scratch();
    this.tabs.push(tab);
    this.activeKey = tab.key;
    return tab;
  }

  /** Activates the tab of a saved request, loading it if it is not open yet. */
  async openSaved(collectionId: string, requestId: string): Promise<void> {
    const open = this.tabs.find((tab) => tab.isFor(collectionId, requestId));
    if (open) {
      this.activeKey = open.key;
      return;
    }
    const result = await this.#api.query({ type: 'request', collectionId, requestId });
    if (result.type !== 'request') {
      throw new Error(`unexpected answer ${result.type}`);
    }
    const tab = new RequestTab(result.request, { collectionId, requestId });
    this.tabs.push(tab);
    this.activeKey = tab.key;
  }

  /** Closes a tab; the last tab is replaced by an empty scratch tab. */
  close(key: string): void {
    const index = this.tabs.findIndex((tab) => tab.key === key);
    if (index < 0) return;
    this.tabs.splice(index, 1);
    if (this.activeKey === key) {
      this.activeKey = (this.tabs[index] ?? this.tabs[index - 1])?.key ?? null;
    }
    if (this.tabs.length === 0) this.openScratch();
  }

  /** Closes the tabs of a collection that is closed. */
  closeCollection(collectionId: string): void {
    for (const tab of this.tabs.filter((tab) => tab.isFor(collectionId))) {
      this.close(tab.key);
    }
  }

  /** Closes tabs whose request no longer exists in the collection. */
  closeMissing(collection: CollectionView): void {
    const existing = requestIds(collection);
    for (const tab of this.tabs.filter((tab) => tab.isFor(collection.id))) {
      if (!existing.has(tab.saved?.requestId ?? '')) this.close(tab.key);
    }
  }

  renamed(collectionId: string, requestId: string, name: string): void {
    this.tabs.find((tab) => tab.isFor(collectionId, requestId))?.renamed(name);
  }

  /** Opens `tab` (e.g. an imported request) and makes it the active one. */
  add(tab: RequestTab): void {
    this.tabs.push(tab);
    this.activeKey = tab.key;
  }

  /**
   * A tab's request as it is sent. `{{variables}}` come from its own
   * variables and, with a `context`, from its collection, folders and
   * environment.
   */
  outgoing(tab: RequestTab, context: SendContext | null = null): OutgoingRequest {
    return {
      method: tab.draft.method,
      url: tab.draft.url,
      headers: tab.draft.headers,
      body: tab.draft.body,
      variables: tab.draft.variables,
      auth: tab.draft.auth,
      certificateCheck: tab.draft.certificateCheck,
      scripts: tab.draft.scripts,
      pathParams: tab.draft.pathParams,
      context,
    };
  }

  /** Sends a tab's request (see `outgoing`). */
  async send(tab: RequestTab, context: SendContext | null = null): Promise<void> {
    tab.error = null;
    try {
      const ack = await this.#api.execute({ type: 'sendRequest', ...this.outgoing(tab, context) });
      if (ack.type !== 'runStarted') {
        throw new Error(`unexpected answer ${ack.type}`);
      }
      const early = this.#early.get(ack.runId);
      this.#early.clear();
      if (early) {
        tab.outcome = early;
        tab.pendingRun = null;
      } else {
        tab.pendingRun = ack.runId;
      }
    } catch (error) {
      tab.error = toUserText(error);
    }
  }

  /** Saves a saved request's tab; resolves to the collection as it is now. */
  async save(tab: RequestTab): Promise<CollectionView | null> {
    if (!tab.saved) return null;
    tab.error = null;
    tab.saving = true;
    try {
      const ack = await this.#api.execute({
        type: 'saveRequest',
        collectionId: tab.saved.collectionId,
        request: tab.draft,
      });
      if (ack.type !== 'collectionChanged') {
        throw new Error(`unexpected answer ${ack.type}`);
      }
      tab.draft.name = tab.draft.name.trim();
      tab.markSaved();
      return ack.collection;
    } catch (error) {
      tab.error = toUserText(error);
      return null;
    } finally {
      tab.saving = false;
    }
  }

  /** Routes a run result to the tab waiting for it (other events are not for tabs). */
  handle(event: AppEvent): void {
    if (event.type !== 'runFinished') return;
    const tab = this.tabs.find((tab) => tab.pendingRun === event.runId);
    if (tab) {
      tab.outcome = event.outcome;
      tab.pendingRun = null;
    } else {
      this.#early.set(event.runId, event.outcome);
    }
  }
}
