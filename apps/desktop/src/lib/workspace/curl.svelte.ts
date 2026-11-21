// cURL: a tab's request as a command (export) and a pasted command as a new
// tab (import, through a dialog).

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { OutgoingRequest, UserText } from '../types';
import { RequestTab } from './request-tab.svelte';
import type { TabsState } from './tabs.svelte';

/** The import dialog: the pasted text and why it could not be read. */
export interface CurlImport {
  text: string;
  error: UserText | null;
}

export class CurlState {
  /** The open import dialog, if any. */
  importing = $state<CurlImport | null>(null);

  readonly #api: Api;
  readonly #tabs: TabsState;

  constructor(api: Api, tabs: TabsState) {
    this.#api = api;
    this.#tabs = tabs;
  }

  /** The command for `request`; `null` (and the tab's error set) on failure. */
  async command(tab: RequestTab, request: OutgoingRequest): Promise<string | null> {
    tab.error = null;
    try {
      const result = await this.#api.query({ type: 'curl', ...request });
      if (result.type !== 'curl') throw new Error(`unexpected answer ${result.type}`);
      return result.command;
    } catch (error) {
      tab.error = toUserText(error);
      return null;
    }
  }

  openImport(): void {
    this.importing = { text: '', error: null };
  }

  closeImport(): void {
    this.importing = null;
  }

  /** Opens the pasted command as a new tab; the dialog stays open on errors. */
  async import(): Promise<void> {
    const importing = this.importing;
    if (!importing) return;
    importing.error = null;
    try {
      const result = await this.#api.query({ type: 'parseCurl', text: importing.text });
      if (result.type !== 'parsedCurl') throw new Error(`unexpected answer ${result.type}`);
      const tab = new RequestTab(result.request, null);
      if (result.ignored.length > 0) {
        tab.notice = { key: 'curl.ignored', args: { options: result.ignored.join(' ') } };
      }
      this.#tabs.add(tab);
      this.importing = null;
    } catch (error) {
      importing.error = toUserText(error);
    }
  }
}
