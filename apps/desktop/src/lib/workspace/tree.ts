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

/** Where an item sits: the id of its collection or folder, its index there and how many items that has. */
export interface Place {
  parentId: string;
  index: number;
  count: number;
}

/** The place of the item `id`, or `null` if it is not in the tree. */
export function locate(collection: CollectionView, id: string): Place | null {
  const walk = (parentId: string, items: TreeItemView[]): Place | null => {
    const index = items.findIndex((item) => item.id === id);
    if (index >= 0) return { parentId, index, count: items.length };
    for (const item of items) {
      if (item.kind !== 'folder') continue;
      const found = walk(item.id, item.items);
      if (found) return found;
    }
    return null;
  };
  return walk(collection.id, collection.items);
}

/** A collection or folder an item can be moved into. */
export interface MoveTarget {
  id: string;
  /** Collection name and folder path, e.g. `Shop / Auth / v2`. */
  label: string;
  /** Items in it now (the position at the end). */
  count: number;
}

/** Where the item `id` can go: the collection and every folder that is not the item or inside it. */
export function moveTargets(collection: CollectionView, id: string): MoveTarget[] {
  const targets: MoveTarget[] = [
    { id: collection.id, label: collection.name, count: collection.items.length },
  ];
  const walk = (items: TreeItemView[], path: string) => {
    for (const item of items) {
      if (item.kind !== 'folder' || item.id === id) continue;
      const label = `${path} / ${item.name}`;
      targets.push({ id: item.id, label, count: item.items.length });
      walk(item.items, label);
    }
  };
  walk(collection.items, collection.name);
  return targets;
}

/** Where a dragged item lands relative to the row it is dropped on. */
export type DropPosition = 'before' | 'after' | 'inside';

/** The drop position for a pointer at `offset` within a row of `height`: folders take the middle half as "inside". */
export function dropPosition(offset: number, height: number, isFolder: boolean): DropPosition {
  const share = height > 0 ? offset / height : 0;
  if (isFolder) {
    if (share < 0.25) return 'before';
    if (share > 0.75) return 'after';
    return 'inside';
  }
  return share < 0.5 ? 'before' : 'after';
}

/**
 * The parent and index for moving `source` next to (or into) `target`, as
 * the move command expects them – the index counts without the moved item.
 * `null` if the move changes nothing or is impossible (into itself).
 */
export function dropPlace(
  collection: CollectionView,
  sourceId: string,
  target: TreeItemView,
  position: DropPosition,
): { parentId: string; index: number } | null {
  const source = locate(collection, sourceId);
  if (!source || sourceId === target.id) return null;
  if (foldersAround(collection, target.id).some((folder) => folder.id === sourceId)) return null;
  if (position === 'inside' && target.kind === 'folder') {
    return { parentId: target.id, index: target.items.length };
  }
  const place = locate(collection, target.id);
  if (!place) return null;
  let index = place.index + (position === 'after' ? 1 : 0);
  if (place.parentId === source.parentId && source.index < index) index -= 1;
  if (place.parentId === source.parentId && index === source.index) return null;
  return { parentId: place.parentId, index };
}
