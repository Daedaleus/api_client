import { describe, expect, it } from 'vitest';

import { fillPathParams, pathParamNames, queryParams, withQueryParams } from './params';

describe('params', () => {
  it('reads query parameters as typed', () => {
    expect(queryParams('{{base}}/items?page=2&q={{term}}&flag&&x=a=b#top')).toEqual([
      { name: 'page', value: '2' },
      { name: 'q', value: '{{term}}' },
      { name: 'flag', value: '' },
      { name: 'x', value: 'a=b' },
    ]);
    expect(queryParams('https://example.com/a')).toEqual([]);
  });

  it('writes query parameters back, keeping the rest of the URL', () => {
    expect(
      withQueryParams('{{base}}/items?old=1#top', [
        { name: 'page', value: '3' },
        { name: '', value: 'ignored' },
        { name: 'flag', value: '' },
      ]),
    ).toBe('{{base}}/items?page=3&flag#top');
    expect(withQueryParams('https://x/a?b=1', [])).toBe('https://x/a');
  });

  it('finds and fills path variables in the path only', () => {
    expect(pathParamNames('http://localhost:8080/users/:id/:id/orders/:order_id?x=:no')).toEqual([
      'id',
      'order_id',
    ]);
    expect(
      fillPathParams('{{base}}/users/:id/:other?x=:id', [
        { name: 'id', value: '42' },
        { name: 'other', value: '' },
      ]),
    ).toBe('{{base}}/users/42/:other?x=:id');
  });
});
