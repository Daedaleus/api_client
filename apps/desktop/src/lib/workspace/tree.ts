// Helpers for the collection tree the backend sends.

import type { CollectionView, TreeItemView } from '../types';

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

/** Last part of a path, for suggesting a collection name from its folder. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? '';
}
