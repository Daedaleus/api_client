// The request history in the sidebar: listed by day, filtered, opened as
// a new tab, cleared. The backend records every sent request (masked) and
// tells us through the `historyChanged` event.

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { HistoryItemView, UserText } from '../types';
import type { Dialogs } from './dialogs.svelte';
import { RequestTab } from './request-tab.svelte';
import type { TabsState } from './tabs.svelte';
import { groupByDay, type HistoryDay } from '../history';

export class HistoryState {
  entries = $state<HistoryItemView[]>([]);
  filter = $state('');
  error = $state<UserText | null>(null);
  /** Shown in the sidebar instead of the collections. */
  visible = $state(false);

  readonly #api: Api;
  readonly #tabs: TabsState;
  readonly #dialogs: Dialogs;

  constructor(api: Api, tabs: TabsState, dialogs: Dialogs) {
    this.#api = api;
    this.#tabs = tabs;
    this.#dialogs = dialogs;
  }

  get days(): HistoryDay[] {
    return groupByDay(this.entries, this.filter);
  }

  /** Shows the history (`true`) or the collections. */
  async show(visible: boolean): Promise<void> {
    this.visible = visible;
    if (visible) await this.refresh();
  }

  /** Reads the entries anew. */
  async refresh(): Promise<void> {
    try {
      const result = await this.#api.query({ type: 'history' });
      if (result.type !== 'history') throw new Error(`unexpected answer ${result.type}`);
      this.entries = result.entries;
    } catch (error) {
      this.error = toUserText(error);
    }
  }

  /** A request was recorded or the history cleared; `error` if recording failed. */
  async changed(error: UserText | null): Promise<void> {
    if (error) this.error = error;
    if (this.visible) await this.refresh();
  }

  /** Opens an entry as a new, unsaved tab with its response. */
  async open(id: string): Promise<void> {
    try {
      const result = await this.#api.query({ type: 'historyEntry', id });
      if (result.type !== 'historyEntry') throw new Error(`unexpected answer ${result.type}`);
      const tab = new RequestTab(result.request, null);
      tab.outcome = result.outcome;
      tab.outcomeRun = null;
      this.#tabs.add(tab);
    } catch (error) {
      this.error = toUserText(error);
    }
  }

  /** Clears the history after asking. */
  async clear(): Promise<void> {
    const ok = await this.#dialogs.confirm(
      { key: 'history.clear', args: {} },
      { key: 'history.clear_confirm', args: {} },
    );
    if (!ok) return;
    try {
      const ack = await this.#api.execute({ type: 'clearHistory' });
      if (ack.type !== 'historyCleared') throw new Error(`unexpected answer ${ack.type}`);
      this.entries = [];
    } catch (error) {
      this.error = toUserText(error);
    }
  }
}
