// Variables of a collection or one of its folders, edited in a dialog.
// (Request variables are part of the request and saved with it.)

import type { Api } from '../api';
import { toUserText } from '../errors';
import type { TreeItemView, UserText, VariableDraft } from '../types';
import type { CollectionsState } from './collections.svelte';
import type { FolderView } from './tree';

/** What the dialog edits. */
export interface VariablesEdit {
  collectionId: string;
  /** The collection itself or a folder in it. */
  itemId: string;
  /** Name of the collection or folder, for the title. */
  name: string;
  /** Whether it is the collection (else a folder). */
  isCollection: boolean;
  /** A copy of the variables, edited in place. */
  variables: VariableDraft[];
}

export class VariablesState {
  editing = $state<VariablesEdit | null>(null);
  error = $state<UserText | null>(null);
  saving = $state(false);

  readonly #api: Api;
  readonly #collections: CollectionsState;

  constructor(api: Api, collections: CollectionsState) {
    this.#api = api;
    this.#collections = collections;
  }

  /** Opens the variables of the collection (`itemId` = its id) or a folder. */
  open(collectionId: string, itemId: string): void {
    const collection = this.#collections.find(collectionId);
    if (!collection) return;
    const target = itemId === collection.id ? collection : findFolder(collection.items, itemId);
    if (!target) return;
    this.error = null;
    this.editing = {
      collectionId,
      itemId,
      name: target.name,
      isCollection: itemId === collection.id,
      variables: $state.snapshot(target.variables),
    };
  }

  close(): void {
    this.editing = null;
    this.error = null;
  }

  /** Saves the variables; the dialog stays open if that fails. */
  async save(): Promise<void> {
    const editing = this.editing;
    if (!editing) return;
    this.error = null;
    this.saving = true;
    try {
      const ack = await this.#api.execute({
        type: 'saveVariables',
        collectionId: editing.collectionId,
        itemId: editing.itemId,
        variables: $state.snapshot(editing.variables),
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

function findFolder(items: TreeItemView[], id: string): FolderView | undefined {
  for (const item of items) {
    if (item.kind !== 'folder') continue;
    if (item.id === id) return item;
    const found = findFolder(item.items, id);
    if (found) return found;
  }
  return undefined;
}
