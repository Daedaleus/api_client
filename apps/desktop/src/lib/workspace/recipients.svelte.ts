// Who a collection's secrets are encrypted for: the list being edited in the
// recipients dialog, and saving it (the app then re-encrypts every secret).

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { UserText } from '../types';
import type { CollectionsState } from './collections.svelte';

/** The recipients of one collection, open in the dialog. */
export interface RecipientsEdit {
  collectionId: string;
  /** The user's own public key. */
  own: string;
  /** Public keys, edited in place. */
  list: string[];
}

export class RecipientsState {
  /** The open dialog, if any. */
  editing = $state<RecipientsEdit | null>(null);
  /** Why loading or saving failed. */
  error = $state<UserText | null>(null);
  saving = $state(false);

  readonly #api: Api;
  readonly #collections: CollectionsState;

  constructor(api: Api, collections: CollectionsState) {
    this.#api = api;
    this.#collections = collections;
  }

  /** Whether the user's own key is missing from the list being edited. */
  get withoutOwnKey(): boolean {
    const editing = this.editing;
    return editing !== null && !editing.list.includes(editing.own);
  }

  async open(collectionId: string): Promise<void> {
    this.error = null;
    try {
      const result = await this.#api.query({ type: 'recipients', collectionId });
      if (result.type !== 'recipients') throw new Error(`unexpected answer ${result.type}`);
      const { own, recipients } = result.recipients;
      // No list yet: the user's own key is the natural start.
      this.editing = { collectionId, own, list: recipients.length > 0 ? recipients : [own] };
    } catch (error) {
      this.#collections.error = toUserText(error);
    }
  }

  /** Adds a key (trimmed, once); false if there was nothing new to add. */
  add(key: string): boolean {
    const trimmed = key.trim();
    if (!this.editing || trimmed === '' || this.editing.list.includes(trimmed)) return false;
    this.editing.list.push(trimmed);
    return true;
  }

  remove(index: number): void {
    this.editing?.list.splice(index, 1);
  }

  close(): void {
    this.editing = null;
    this.error = null;
  }

  /** Saves the list; the dialog stays open if that fails. */
  async save(): Promise<void> {
    const editing = this.editing;
    if (!editing) return;
    this.error = null;
    this.saving = true;
    try {
      const ack = await this.#api.execute({
        type: 'setRecipients',
        collectionId: editing.collectionId,
        recipients: [...editing.list],
      });
      if (ack.type !== 'collectionChanged') throw new Error(`unexpected answer ${ack.type}`);
      this.#collections.apply(ack.collection);
      this.close();
    } catch (error) {
      this.error = toUserText(error);
    } finally {
      this.saving = false;
    }
  }
}
