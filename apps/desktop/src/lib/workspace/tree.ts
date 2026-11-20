// Helpers for the collection tree the backend sends.

import type { CollectionView, TreeItemView } from '../types';

export type FolderView = Extract<TreeItemView, { kind: 'folder' }>;

/** Ids of all requests in the tree. */
export function requestIds(collection: CollectionView): Set<string> {
  const ids = new Set<string>();
  const walk = (items: TreeItemView[]) => {
    for (const item of items) {
      if (item.kind === 'request') ids.add(item.id);
      else walk(item.items);
    }
  };
  walk(collection.items);
  return ids;
}

/** The folders around the item `id`, outermost first. */
export function foldersAround(collection: CollectionView, id: string): FolderView[] {
  const walk = (items: TreeItemView[], path: FolderView[]): FolderView[] | null => {
    for (const item of items) {
      if (item.id === id) return path;
      if (item.kind === 'folder') {
        const found = walk(item.items, [...path, item]);
        if (found) return found;
      }
    }
    return null;
  };
  return walk(collection.items, []) ?? [];
}

/** Last part of a path, for suggesting a collection name from its folder. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? '';
}
