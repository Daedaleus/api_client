import { describe, expect, it } from 'vitest';

import { JsonPathError, queryJson } from './jsonpath';

const shop = {
  name: 'Tea shop',
  'odd key': 1,
  items: [
    { id: 1, name: 'Green', tags: ['a'] },
    { id: 2, name: 'Black', tags: [] },
    { id: 3, name: 'Oolong', tags: ['b', 'c'] },
  ],
  owner: { name: 'Ada', address: { city: 'Berlin' } },
};

describe('queryJson', () => {
  it('selects members and elements', () => {
    expect(queryJson(shop, '$')).toEqual([shop]);
    expect(queryJson(shop, '$.name')).toEqual(['Tea shop']);
    expect(queryJson(shop, " $['odd key'] ")).toEqual([1]);
    expect(queryJson(shop, '$["owner"].address.city')).toEqual(['Berlin']);
    expect(queryJson(shop, '$.items[0].name')).toEqual(['Green']);
    expect(queryJson(shop, '$.items[-1].id')).toEqual([3]);
    expect(queryJson(shop, '$.items[9]')).toEqual([]);
    expect(queryJson(shop, '$.missing.deeper')).toEqual([]);
    expect(queryJson(shop, '$.name.length')).toEqual([]);
  });

  it('selects all, below and several', () => {
    expect(queryJson(shop, '$.items[*].id')).toEqual([1, 2, 3]);
    expect(queryJson(shop, '$.owner.*')).toEqual(['Ada', { city: 'Berlin' }]);
    expect(queryJson(shop, '$..name')).toEqual(['Tea shop', 'Green', 'Black', 'Oolong', 'Ada']);
    expect(queryJson(shop, '$..tags[*]')).toEqual(['a', 'b', 'c']);
    expect(queryJson(shop, '$.items[0,2].id')).toEqual([1, 3]);
    expect(queryJson(shop, "$.owner['name','missing']")).toEqual(['Ada']);
    expect(queryJson([1, [2, [3]]], '$..*')).toEqual([1, [2, [3]], 2, [3], 3]);
  });

  it('slices', () => {
    const list = [0, 1, 2, 3, 4, 5];
    expect(queryJson(list, '$[1:3]')).toEqual([1, 2]);
    expect(queryJson(list, '$[:2]')).toEqual([0, 1]);
    expect(queryJson(list, '$[4:]')).toEqual([4, 5]);
    expect(queryJson(list, '$[-2:]')).toEqual([4, 5]);
    expect(queryJson(list, '$[::2]')).toEqual([0, 2, 4]);
    expect(queryJson(list, '$[::-2]')).toEqual([5, 3, 1]);
    expect(queryJson(list, '$[1:100]')).toHaveLength(5);
    expect(queryJson(list, '$[::0]')).toEqual([]);
    expect(queryJson({ a: 1 }, '$[0:1]')).toEqual([]);
  });

  it('explains paths it cannot use', () => {
    const error = (path: string) => {
      try {
        queryJson(shop, path);
      } catch (thrown) {
        return thrown instanceof JsonPathError ? [thrown.key, thrown.args] : thrown;
      }
      return null;
    };
    expect(error('items')).toEqual(['jsonpath.start', {}]);
    expect(error('$items')).toEqual(['jsonpath.unexpected', { at: '2' }]);
    expect(error('$.items[0')).toEqual(['jsonpath.unclosed', { at: '8' }]);
    expect(error('$.items[?(@.id > 1)]')).toEqual([
      'jsonpath.selector',
      { selector: '?(@.id > 1)', at: '8' },
    ]);
    expect(error('$.')).toEqual(['jsonpath.unexpected', { at: '3' }]);
    expect(error("$['a]b']")).toBeNull();
  });
});
