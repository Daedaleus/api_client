// Which environment is chosen per collection, remembered between starts.

import { type KeyValueStorage, readJson, writeJson } from '../storage';
import type { CollectionView, EnvironmentDraft, EnvironmentRef } from '../types';

const STORAGE_KEY = 'activeEnvironments';

type Choices = Record<string, string>;

function isChoices(value: unknown): value is Choices {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export class EnvironmentChoice {
  /** Environment id per collection id. */
  #chosen = $state<Choices>({});
  readonly #storage: KeyValueStorage | null;

  constructor(storage: KeyValueStorage | null) {
    this.#storage = storage;
    this.#chosen = readJson(storage, STORAGE_KEY, {}, isChoices);
  }

  /** The chosen environment of `collection`, if it still exists. */
  of(collection: CollectionView | undefined): EnvironmentDraft | null {
    if (!collection) return null;
    const id = this.#chosen[collection.id];
    return collection.environments.find((environment) => environment.id === id) ?? null;
  }

  /** Chooses an environment (`null` = none) for a collection. */
  choose(collectionId: string, environmentId: string | null): void {
    const next = { ...this.#chosen };
    if (environmentId) next[collectionId] = environmentId;
    else delete next[collectionId];
    this.#chosen = next;
    writeJson(this.#storage, STORAGE_KEY, next);
  }

  /** What to send with for a request of `collection`. */
  reference(collection: CollectionView | undefined): EnvironmentRef | null {
    const environment = this.of(collection);
    return collection && environment
      ? { collectionId: collection.id, environmentId: environment.id }
      : null;
  }
}
