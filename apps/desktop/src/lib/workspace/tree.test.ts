import { describe, expect, it } from 'vitest';

import { shop } from '../../test/fake-api';
import type { CollectionView, TreeItemView } from '../types';
import { dropPlace, dropPosition, locate, moveTargets } from './tree';

/** Shop with a third top-level request and a subfolder in Auth. */
function tree(): CollectionView {
  const collection = shop();
  const auth = collection.items[0] as Extract<TreeItemView, { kind: 'folder' }>;
  auth.items.push({ ...auth, id: 'f2', name: 'v2', items: [] });
  collection.items.push({
    kind: 'request',
    id: 'r3',
    name: 'Status',
    method: 'GET',
    readOnly: false,
  });
  return collection;
}

const item = (collection: CollectionView, id: string): TreeItemView => {
  const walk = (items: TreeItemView[]): TreeItemView | undefined =>
    items.find((candidate) => candidate.id === id) ??
    items.flatMap((candidate) => (candidate.kind === 'folder' ? [walk(candidate.items)] : []))[0];
  return walk(collection.items)!;
};

describe('tree places', () => {
  it('locates items and lists move targets outside the item', () => {
    const collection = tree();
    expect(locate(collection, 'r1')).toEqual({ parentId: 'f1', index: 0, count: 2 });
    expect(locate(collection, 'r3')).toEqual({ parentId: 'c1', index: 2, count: 3 });
    expect(locate(collection, 'nope')).toBeNull();
    expect(moveTargets(collection, 'r1').map((target) => target.label)).toEqual([
      'Shop',
      'Shop / Auth',
      'Shop / Auth / v2',
    ]);
    expect(moveTargets(collection, 'f1').map((target) => target.id)).toEqual(['c1']);
  });

  it('tells where on a row a drop lands', () => {
    expect(dropPosition(2, 20, false)).toBe('before');
    expect(dropPosition(15, 20, false)).toBe('after');
    expect(dropPosition(2, 20, true)).toBe('before');
    expect(dropPosition(10, 20, true)).toBe('inside');
    expect(dropPosition(18, 20, true)).toBe('after');
    expect(dropPosition(5, 0, false)).toBe('before');
  });

  it('turns a drop into parent and index without the moved item', () => {
    const collection = tree();
    // Health (index 1) after Status (index 2): index 2 once Health is out.
    expect(dropPlace(collection, 'r2', item(collection, 'r3'), 'after')).toEqual({
      parentId: 'c1',
      index: 2,
    });
    // Status before Auth: first.
    expect(dropPlace(collection, 'r3', item(collection, 'f1'), 'before')).toEqual({
      parentId: 'c1',
      index: 0,
    });
    // Into a folder: at its end; next to an item in another folder.
    expect(dropPlace(collection, 'r3', item(collection, 'f1'), 'inside')).toEqual({
      parentId: 'f1',
      index: 2,
    });
    expect(dropPlace(collection, 'r2', item(collection, 'r1'), 'after')).toEqual({
      parentId: 'f1',
      index: 1,
    });
    // Nowhere new, onto itself, into its own subfolder.
    expect(dropPlace(collection, 'r2', item(collection, 'r3'), 'before')).toBeNull();
    expect(dropPlace(collection, 'r2', item(collection, 'r2'), 'after')).toBeNull();
    expect(dropPlace(collection, 'f1', item(collection, 'f2'), 'inside')).toBeNull();
    expect(dropPlace(collection, 'nope', item(collection, 'r2'), 'after')).toBeNull();
  });
});
