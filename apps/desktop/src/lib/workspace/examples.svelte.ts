// Response examples of a saved request: loaded when the tab shows them,
// kept from the response shown in the tab, removed after asking. The
// backend keeps them in a file next to the request.

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { CommandAck } from '../types';
import type { Dialogs } from './dialogs.svelte';
import type { RequestTab } from './request-tab.svelte';

export class ExamplesState {
  readonly #api: Api;
  readonly #dialogs: Dialogs;

  constructor(api: Api, dialogs: Dialogs) {
    this.#api = api;
    this.#dialogs = dialogs;
  }

  /** Whether the shown response can be kept as an example of the tab's request. */
  canSave(tab: RequestTab): boolean {
    return tab.saved !== null && tab.outcomeRun !== null && tab.outcome?.kind === 'completed';
  }

  /** Loads the examples of a saved tab (once). */
  async load(tab: RequestTab): Promise<void> {
    const saved = tab.saved;
    if (!saved || tab.examples !== null) return;
    try {
      const result = await this.#api.query({ type: 'examples', ...saved });
      if (result.type !== 'examples') throw new Error(`unexpected answer ${result.type}`);
      tab.examples = result.examples;
    } catch (error) {
      tab.error = toUserText(error);
    }
  }

  /** Keeps the shown response as an example, named by the user. */
  async saveFrom(tab: RequestTab): Promise<void> {
    const saved = tab.saved;
    const outcome = tab.outcome;
    if (!saved || tab.outcomeRun === null || outcome?.kind !== 'completed') return;
    const name = await this.#dialogs.askText(
      { key: 'examples.save_title', args: {} },
      { key: 'examples.name', args: {} },
      `${outcome.status} ${outcome.reason ?? ''}`.trim(),
    );
    if (name === null) return;
    await this.#change(tab, {
      type: 'saveExample',
      ...saved,
      runId: tab.outcomeRun,
      name,
    });
  }

  /** Removes example `index` after asking. */
  async remove(tab: RequestTab, index: number): Promise<void> {
    const saved = tab.saved;
    const example = tab.examples?.[index];
    if (!saved || !example) return;
    const confirmed = await this.#dialogs.confirm(
      { key: 'examples.delete_title', args: { name: example.name } },
      { key: 'examples.delete_message', args: {} },
    );
    if (!confirmed) return;
    await this.#change(tab, { type: 'deleteExample', ...saved, index });
  }

  async #change(tab: RequestTab, command: Parameters<Api['execute']>[0]): Promise<void> {
    tab.error = null;
    try {
      const ack: CommandAck = await this.#api.execute(command);
      if (ack.type !== 'examplesChanged') throw new Error(`unexpected answer ${ack.type}`);
      tab.examples = ack.examples;
    } catch (error) {
      tab.error = toUserText(error);
    }
  }
}
