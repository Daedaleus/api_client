// The cookie manager: the cookies of one collection's jar (in memory in the
// backend), editing one of them, deleting and clearing.

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { CommandAck, CookieKeyView, CookieView, UserText } from '../types';
import type { Dialogs } from './dialogs.svelte';

/** The jar shown: a collection's (its id and name). */
export interface CookieJarShown {
  collectionId: string;
  name: string;
}

/** A cookie in the edit form; `original` is `null` for a new one. */
export interface CookieEdit {
  original: CookieKeyView | null;
  draft: CookieView;
}

/** What identifies `cookie` in its jar. */
export function keyOf(cookie: CookieView): CookieKeyView {
  return { name: cookie.name, domain: cookie.domain, path: cookie.path };
}

export class CookiesState {
  /** The jar shown in the dialog, if it is open. */
  shown = $state<CookieJarShown | null>(null);
  cookies = $state<CookieView[]>([]);
  /** The cookie in the edit form, if any. */
  editing = $state<CookieEdit | null>(null);
  /** Why loading or a change failed. */
  error = $state<UserText | null>(null);

  readonly #api: Api;
  readonly #dialogs: Dialogs;

  constructor(api: Api, dialogs: Dialogs) {
    this.#api = api;
    this.#dialogs = dialogs;
  }

  async open(collectionId: string, name: string): Promise<void> {
    this.shown = { collectionId, name };
    this.editing = null;
    this.error = null;
    this.cookies = [];
    try {
      const result = await this.#api.query({ type: 'cookies', collectionId });
      if (result.type !== 'cookies') throw new Error(`unexpected answer ${result.type}`);
      this.cookies = result.cookies;
    } catch (error) {
      this.error = toUserText(error);
    }
  }

  close(): void {
    this.shown = null;
    this.editing = null;
    this.error = null;
  }

  /** A new session cookie, for the domain of the first cookie if any. */
  add(): void {
    this.editing = {
      original: null,
      draft: {
        name: '',
        value: '',
        domain: this.cookies[0]?.domain ?? '',
        hostOnly: true,
        path: '/',
        expires: null,
        secure: false,
        httpOnly: false,
      },
    };
    this.error = null;
  }

  edit(cookie: CookieView): void {
    this.editing = { original: keyOf(cookie), draft: { ...cookie } };
    this.error = null;
  }

  cancelEdit(): void {
    this.editing = null;
    this.error = null;
  }

  /** Saves the edit form; it stays open if the cookie is not usable. */
  async save(): Promise<void> {
    const editing = this.editing;
    if (!editing) return;
    const done = await this.#change({
      type: 'saveCookie',
      collectionId: this.shown?.collectionId ?? null,
      original: editing.original,
      cookie: { ...editing.draft },
    });
    if (done) this.editing = null;
  }

  async remove(cookie: CookieView): Promise<void> {
    await this.#change({
      type: 'deleteCookie',
      collectionId: this.shown?.collectionId ?? null,
      cookie: keyOf(cookie),
    });
  }

  /** Removes all cookies of the jar after asking. */
  async clear(): Promise<void> {
    const confirmed = await this.#dialogs.confirm(
      { key: 'cookies.clear_title', args: {} },
      { key: 'cookies.clear_message', args: {} },
    );
    if (!confirmed) return;
    this.editing = null;
    await this.#change({ type: 'clearCookies', collectionId: this.shown?.collectionId ?? null });
  }

  /** Runs a change; whether it worked (the list is then current). */
  async #change(command: Parameters<Api['execute']>[0]): Promise<boolean> {
    this.error = null;
    try {
      const ack: CommandAck = await this.#api.execute(command);
      if (ack.type !== 'cookiesChanged') throw new Error(`unexpected answer ${ack.type}`);
      this.cookies = ack.cookies;
      return true;
    } catch (error) {
      this.error = toUserText(error);
      return false;
    }
  }
}
