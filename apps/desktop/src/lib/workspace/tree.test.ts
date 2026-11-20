import { describe, expect, it } from 'vitest';

import { shop } from '../../test/fake-api';
import { toUserText } from '../errors';
import { baseName, foldersAround, requestIds } from './tree';

describe('tree helpers', () => {
  it('collects the ids of all requests', () => {
    expect([...requestIds(shop())].sort()).toEqual(['r1', 'r2']);
  });

  it('takes the last part of a path', () => {
    expect(baseName('C:\\apis\\shop')).toBe('shop');
    expect(baseName('/home/me/billing/')).toBe('billing');
    expect(baseName('')).toBe('');
  });
});

describe('toUserText', () => {
  it('keeps user texts and wraps anything else', () => {
    expect(toUserText({ key: 'a', args: {} })).toEqual({ key: 'a', args: {} });
    expect(toUserText(new Error('boom'))).toEqual({
      key: 'error.unexpected',
      args: { detail: 'boom' },
    });
    expect(toUserText('command not found').args).toEqual({ detail: 'command not found' });
  });

  it('finds the folders around an item, outermost first', () => {
    const collection = shop();
    expect(foldersAround(collection, 'r1').map((folder) => folder.name)).toEqual(['Auth']);
    expect(foldersAround(collection, 'r2')).toEqual([]);
    expect(foldersAround(collection, 'missing')).toEqual([]);
  });
});
