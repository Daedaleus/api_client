// State of the request editor: what the user entered, the run in flight and
// its outcome. Talks to the backend only through `Api`.

import type { Api } from './api';
import { type AppEvent, type RunOutcome, type UserText, isUserText } from './types';

export const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;

/** Turns anything a failed `execute` rejects with into a display text. */
export function toUserText(error: unknown): UserText {
  if (isUserText(error)) {
    return error;
  }
  const detail = error instanceof Error ? error.message : String(error);
  return { key: 'error.unexpected', args: { detail } };
}

export class RequestController {
  method = $state<string>('GET');
  url = $state('');
  /** Run whose result is awaited; `null` when idle. */
  pendingRun = $state<number | null>(null);
  outcome = $state<RunOutcome | null>(null);
  /** Why the last send could not start. */
  error = $state<UserText | null>(null);

  readonly #api: Api;
  /** Results that arrived before their acknowledgement; never rendered, so not reactive. */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  readonly #early = new Map<number, RunOutcome>();

  constructor(api: Api) {
    this.#api = api;
  }

  get sending(): boolean {
    return this.pendingRun !== null;
  }

  /** Starts listening for results; resolves to an unsubscribe function. */
  connect(): Promise<() => void> {
    return this.#api.onEvent((event) => this.#handle(event));
  }

  async send(): Promise<void> {
    this.error = null;
    try {
      const ack = await this.#api.execute({
        type: 'sendRequest',
        method: this.method,
        url: this.url,
      });
      const early = this.#early.get(ack.runId);
      this.#early.clear();
      if (early) {
        this.outcome = early;
        this.pendingRun = null;
      } else {
        this.pendingRun = ack.runId;
      }
    } catch (error) {
      this.error = toUserText(error);
    }
  }

  #handle(event: AppEvent): void {
    if (event.runId === this.pendingRun) {
      this.outcome = event.outcome;
      this.pendingRun = null;
    } else if (this.pendingRun === null || event.runId > this.pendingRun) {
      this.#early.set(event.runId, event.outcome);
    }
  }
}
