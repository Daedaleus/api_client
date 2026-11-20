import { describe, expect, it } from 'vitest';

import { FakeApi, changed, memoryStorage, shop } from '../../test/fake-api';
import { CollectionsState } from './collections.svelte';

describe('CollectionsState', () => {
  it('takes over the tree a command answers with and remembers its folder', async () => {
    const api = new FakeApi();
    const storage = memoryStorage();
    const state = new CollectionsState(api, storage);
    api.answers.push(changed(shop(), 'r9'));

    const result = await state.run({ type: 'openCollection', path: 'C:\\apis\\shop' });
    expect(result?.createdId).toBe('r9');
    expect(state.collections.map((collection) => collection.name)).toEqual(['Shop']);
    expect(storage.data.get('openCollections')).toBe(JSON.stringify(['C:\\apis\\shop']));

    const renamed = { ...shop(), name: 'Shop v2' };
    state.apply(renamed);
    expect(state.collections).toHaveLength(1);
    expect(state.find('c1')?.name).toBe('Shop v2');
  });

  it('reports failures and unexpected answers', async () => {
    const api = new FakeApi();
    const state = new CollectionsState(api, null);
    api.answers.push({ fail: { key: 'error.not_a_collection', args: { path: 'x' } } });
    expect(await state.run({ type: 'openCollection', path: 'x' })).toBeNull();
    expect(state.error?.key).toBe('error.not_a_collection');

    api.answers.push({ type: 'runStarted', runId: 1 });
    expect(await state.run({ type: 'openCollection', path: 'x' })).toBeNull();
    expect(state.error?.args.detail).toBe('unexpected answer runStarted');
  });

  it('reopens the collections of the last session', async () => {
    const api = new FakeApi();
    const storage = memoryStorage({ openCollections: JSON.stringify(['a', 'gone', 42]) });
    const state = new CollectionsState(api, storage);
    api.answers.push(changed(shop()), { fail: { key: 'error.not_a_collection', args: {} } });

    await state.restore();
    expect(api.commands).toEqual([
      { type: 'openCollection', path: 'a' },
      { type: 'openCollection', path: 'gone' },
    ]);
    // Only what opened is remembered.
    expect(storage.data.get('openCollections')).toBe(JSON.stringify(['C:\\apis\\shop']));
  });

  it('survives broken or missing storage', async () => {
    const api = new FakeApi();
    await new CollectionsState(api, memoryStorage({ openCollections: '{oops' })).restore();
    await new CollectionsState(api, memoryStorage({ openCollections: '"x"' })).restore();
    const throwing = {
      getItem: () => {
        throw new Error('denied');
      },
      setItem: () => {
        throw new Error('denied');
      },
    };
    const state = new CollectionsState(api, throwing);
    await state.restore();
    state.apply(shop());
    expect(state.collections).toHaveLength(1);
    expect(api.commands).toHaveLength(0);
  });

  it('closes collections, also when the backend already forgot them', async () => {
    const api = new FakeApi();
    const state = new CollectionsState(api, null);
    state.apply(shop());
    api.answers.push({ fail: { key: 'error.unknown_collection', args: {} } });
    await state.close('c1');
    expect(state.collections).toHaveLength(0);
    expect(state.error?.key).toBe('error.unknown_collection');

    state.apply(shop());
    api.answers.push({ type: 'collectionClosed', collectionId: 'c1' });
    await state.close('c1');
    expect(state.error).toBeNull();
  });
});
