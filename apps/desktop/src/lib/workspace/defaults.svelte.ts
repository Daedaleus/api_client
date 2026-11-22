// What a collection or one of its folders passes on to its requests –
// variables, authentication, certificate check and scripts – edited in a
// dialog. (A request's own are
// part of the request and saved with it.)

import type { Api } from '../api';
import { toUserText } from '../errors';
import type {
  AuthDraft,
  CertificateCheckView,
  ScriptsDraft,
  TreeItemView,
  UserText,
  VariableDraft,
} from '../types';
import type { CollectionsState } from './collections.svelte';
import type { FolderView } from './tree';

/** What the dialog edits. */
export interface DefaultsEdit {
  collectionId: string;
  /** The collection itself or a folder in it. */
  itemId: string;
  /** Name of the collection or folder, for the title. */
  name: string;
  /** Whether it is the collection (else a folder). */
  isCollection: boolean;
  /** A copy of the variables, edited in place. */
  variables: VariableDraft[];
  /** A copy of the authentication. */
  auth: AuthDraft;
  /** The certificate check. */
  certificateCheck: CertificateCheckView;
  /** A copy of the scripts around its requests. */
  scripts: ScriptsDraft;
}

export class DefaultsState {
  editing = $state<DefaultsEdit | null>(null);
  error = $state<UserText | null>(null);
  saving = $state(false);

  readonly #api: Api;
  readonly #collections: CollectionsState;

  constructor(api: Api, collections: CollectionsState) {
    this.#api = api;
    this.#collections = collections;
  }

  /** Opens the collection (`itemId` = its id) or a folder. */
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
      auth: $state.snapshot(target.auth),
      certificateCheck: target.certificateCheck,
      scripts: $state.snapshot(target.scripts),
    };
  }

  close(): void {
    this.editing = null;
    this.error = null;
  }

  /** Saves; the dialog stays open if that fails. */
  async save(): Promise<void> {
    const editing = this.editing;
    if (!editing) return;
    this.error = null;
    this.saving = true;
    try {
      const ack = await this.#api.execute({
        type: 'saveDefaults',
        collectionId: editing.collectionId,
        itemId: editing.itemId,
        variables: $state.snapshot(editing.variables),
        auth: $state.snapshot(editing.auth),
        certificateCheck: editing.certificateCheck,
        scripts: $state.snapshot(editing.scripts),
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
