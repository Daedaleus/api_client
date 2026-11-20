// The open collections: their trees as the backend reports them, and which
// folders to open again on the next start.

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { ChangedCollection, CollectionView, Command, UserText } from '../types';

const STORAGE_KEY = 'openCollections';

type PathStorage = Pick<Storage, 'getItem' | 'setItem'>;

export class CollectionsState {
  collections = $state<CollectionView[]>([]);
  /** Why the last collection command failed. */
  error = $state<UserText | null>(null);

  readonly #api: Api;
  readonly #storage: PathStorage | null;

  constructor(api: Api, storage: PathStorage | null) {
    this.#api = api;
    this.#storage = storage;
  }

  find(collectionId: string): CollectionView | undefined {
    return this.collections.find((collection) => collection.id === collectionId);
  }

  /** Opens the collections of the last session; ones that fail are dropped. */
  async restore(): Promise<void> {
    for (const path of this.#storedPaths()) {
      await this.run({ type: 'openCollection', path });
    }
    this.#persist();
  }

  /**
   * Runs a collection command and takes over the tree it answers with.
   * Resolves to the changed collection, or `null` if the command failed
   * (then `error` says why).
   */
  async run(command: Command): Promise<ChangedCollection | null> {
    this.error = null;
    try {
      const ack = await this.#api.execute(command);
      if (ack.type !== 'collectionChanged') {
        throw new Error(`unexpected answer ${ack.type}`);
      }
      this.apply(ack.collection);
      return { collection: ack.collection, createdId: ack.createdId ?? null };
    } catch (error) {
      this.error = toUserText(error);
      return null;
    }
  }

  async close(collectionId: string): Promise<void> {
    this.error = null;
    try {
      await this.#api.execute({ type: 'closeCollection', collectionId });
    } catch (error) {
      // Already closed in the backend: still remove it here.
      this.error = toUserText(error);
    }
    this.collections = this.collections.filter((collection) => collection.id !== collectionId);
    this.#persist();
  }

  /** Takes over a collection tree (adds it if it is new). */
  apply(collection: CollectionView): void {
    const index = this.collections.findIndex((open) => open.id === collection.id);
    if (index < 0) this.collections.push(collection);
    else this.collections[index] = collection;
    this.#persist();
  }

  #storedPaths(): string[] {
    try {
      const value: unknown = JSON.parse(this.#storage?.getItem(STORAGE_KEY) ?? '[]');
      return Array.isArray(value) ? value.filter((path) => typeof path === 'string') : [];
    } catch {
      return [];
    }
  }

  #persist(): void {
    try {
      this.#storage?.setItem(
        STORAGE_KEY,
        JSON.stringify(this.collections.map((collection) => collection.root)),
      );
    } catch {
      // Not remembered for the next start (storage disabled) – still open now.
    }
  }
}
