import { describe, expect, it } from 'vitest';

import { shop } from '../test/fake-api';
import { filterTree, searchRequests } from './search';
import type { CollectionView, TreeItemView } from './types';

function billing(): CollectionView {
  return {
    ...shop(),
    id: 'c2',
    name: 'Billing',
    items: [
      {
        kind: 'request',
        id: 'b1',
        name: 'Invoices',
        method: 'GET',
        url: '{{baseUrl}}/invoices',
        readOnly: false,
      },
      {
        kind: 'request',
        id: 'b2',
        name: 'Create login token',
        method: 'POST',
        url: '{{baseUrl}}/tokens',
        readOnly: false,
      },
    ],
  };
}

const names = (items: TreeItemView[]): string[] =>
  items.map((item) =>
    item.kind === 'folder' ? `${item.name}/${names(item.items).join(',')}` : item.name,
  );

describe('search', () => {
  it('finds requests by name, method, URL, folder and collection, best first', () => {
    const collections = [shop(), billing()];
    const login = searchRequests(collections, 'login');
    expect(login.map((hit) => hit.request.name)).toEqual(['Login', 'Create login token']);
    expect(login[0]).toMatchObject({ collectionId: 'c1', path: 'Shop / Auth' });
    expect(searchRequests(collections, 'post tokens').map((hit) => hit.request.id)).toEqual(['b2']);
    expect(searchRequests(collections, 'AUTH').map((hit) => hit.request.id)).toEqual(['r1']);
    expect(searchRequests(collections, 'billing').map((hit) => hit.request.id)).toEqual([
      'b1',
      'b2',
    ]);
    expect(searchRequests(collections, '  ')).toEqual([]);
    expect(searchRequests(collections, 'e', 1)).toHaveLength(1);
    // The exact name before names that only start with it.
    const copy = billing();
    copy.items.unshift({
      ...(shop().items[1] as TreeItemView),
      id: 'h2',
      name: 'Health (copy)',
    } as TreeItemView);
    expect(searchRequests([copy, shop()], 'health').map((hit) => hit.request.name)).toEqual([
      'Health',
      'Health (copy)',
    ]);
  });

  it('filters the tree, keeping folders around matches', () => {
    const items = shop().items;
    expect(names(filterTree(items, ''))).toEqual(['Auth/Login', 'Health']);
    expect(names(filterTree(items, 'health'))).toEqual(['Health']);
    expect(names(filterTree(items, '/auth/login'))).toEqual(['Auth/Login']);
    expect(names(filterTree(items, 'auth'))).toEqual(['Auth/Login']);
    expect(names(filterTree(items, 'nothing'))).toEqual([]);
  });
});
