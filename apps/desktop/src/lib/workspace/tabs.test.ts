import { describe, expect, it } from 'vitest';

import { FakeApi, OK, changed, draft, shop } from '../../test/fake-api';
import { TabsState } from './tabs.svelte';

function setup() {
  const api = new FakeApi();
  const tabs = new TabsState(api);
  void api.onEvent((event) => tabs.handle(event));
  return { api, tabs };
}

describe('TabsState', () => {
  it('starts with one empty scratch tab', () => {
    const { tabs } = setup();
    expect(tabs.tabs).toHaveLength(1);
    expect(tabs.active?.saved).toBeNull();
    expect(tabs.active?.dirty).toBe(false);
  });

  it('opens saved requests once and activates them', async () => {
    const { api, tabs } = setup();
    await tabs.openSaved('c1', 'r1');
    await tabs.openSaved('c1', 'r2');
    await tabs.openSaved('c1', 'r1');
    expect(api.queries).toHaveLength(2);
    expect(tabs.tabs).toHaveLength(3);
    expect(tabs.active?.saved).toEqual({ collectionId: 'c1', requestId: 'r1' });
    expect(tabs.active?.draft.name).toBe('Request r1');
  });

  it('refuses unexpected query answers', async () => {
    const { api, tabs } = setup();
    api.queryAnswers.push({ type: 'collection', collection: shop() });
    await expect(tabs.openSaved('c1', 'r1')).rejects.toThrow('unexpected answer collection');
  });

  it('closes tabs and keeps one tab open', async () => {
    const { tabs } = setup();
    const scratch = tabs.active!;
    await tabs.openSaved('c1', 'r1');
    await tabs.openSaved('c2', 'r9');
    tabs.closeCollection('c1');
    expect(tabs.tabs.map((tab) => tab.saved?.requestId ?? 'scratch')).toEqual(['scratch', 'r9']);
    tabs.close(tabs.active!.key);
    expect(tabs.active?.key).toBe(scratch.key);
    tabs.close(scratch.key);
    expect(tabs.tabs).toHaveLength(1);
    expect(tabs.active?.key).not.toBe(scratch.key);
    tabs.close('unknown');
    expect(tabs.tabs).toHaveLength(1);
  });

  it('closes tabs of requests that are gone', async () => {
    const { tabs } = setup();
    await tabs.openSaved('c1', 'r1');
    await tabs.openSaved('c1', 'gone');
    tabs.closeMissing(shop());
    expect(tabs.tabs.map((tab) => tab.saved?.requestId ?? 'scratch')).toEqual(['scratch', 'r1']);
  });

  it('sends the active request and shows the matching result', async () => {
    const { api, tabs } = setup();
    const tab = tabs.active!;
    tab.draft.method = 'POST';
    tab.draft.url = 'example.com';
    await tabs.send(tab);
    expect(api.commands).toEqual([
      {
        type: 'sendRequest',
        method: 'POST',
        url: 'example.com',
        headers: [],
        body: { kind: 'none' },
        variables: [],
        auth: { kind: 'inherit' },
        context: null,
      },
    ]);
    expect(tab.sending).toBe(true);
    api.emit(1, OK);
    expect(tab.sending).toBe(false);
    expect(tab.outcome).toEqual(OK);
  });

  it('routes results to the tab that sent', async () => {
    const { api, tabs } = setup();
    const first = tabs.active!;
    const second = tabs.openScratch();
    await tabs.send(first);
    await tabs.send(second);
    api.emit(2, OK);
    expect(second.outcome).toEqual(OK);
    expect(first.sending).toBe(true);
  });

  it('handles results that arrive before the acknowledgement', async () => {
    const { api, tabs } = setup();
    api.emitBeforeAck = OK;
    await tabs.send(tabs.active!);
    expect(tabs.active?.outcome).toEqual(OK);
    expect(tabs.active?.sending).toBe(false);
  });

  it('shows why a send could not start', async () => {
    const { api, tabs } = setup();
    api.answers.push({ fail: { key: 'error.url_empty', args: {} } });
    await tabs.send(tabs.active!);
    expect(tabs.active?.error).toEqual({ key: 'error.url_empty', args: {} });

    api.answers.push(changed(shop()));
    await tabs.send(tabs.active!);
    expect(tabs.active?.error?.args).toEqual({ detail: 'unexpected answer collectionChanged' });
  });

  it('saves saved requests and reports the collection', async () => {
    const { api, tabs } = setup();
    await tabs.openSaved('c1', 'r2');
    const tab = tabs.active!;
    tab.draft.url = 'https://example.com/v2';
    tab.draft.name = '  Health v2 ';
    expect(tab.dirty).toBe(true);

    api.answers.push(changed(shop()));
    const collection = await tabs.save(tab);
    expect(collection?.id).toBe('c1');
    expect(api.commands.at(-1)).toMatchObject({ type: 'saveRequest', collectionId: 'c1' });
    expect(tab.draft.name).toBe('Health v2');
    expect(tab.dirty).toBe(false);
    expect(tab.saving).toBe(false);
  });

  it('keeps changes and shows the error when saving fails', async () => {
    const { api, tabs } = setup();
    await tabs.openSaved('c1', 'r2');
    const tab = tabs.active!;
    tab.draft.url = 'changed';
    api.answers.push({ fail: { key: 'error.store_read_only', args: { path: 'x' } } });
    expect(await tabs.save(tab)).toBeNull();
    expect(tab.dirty).toBe(true);
    expect(tab.error?.key).toBe('error.store_read_only');

    api.answers.push({ type: 'runStarted', runId: 9 });
    expect(await tabs.save(tab)).toBeNull();
    expect(tab.error?.key).toBe('error.unexpected');
  });

  it('does not save scratch tabs', async () => {
    const { api, tabs } = setup();
    expect(await tabs.save(tabs.active!)).toBeNull();
    expect(api.commands).toHaveLength(0);
  });

  it('takes over names changed in the tree without losing other edits', async () => {
    const { tabs } = setup();
    await tabs.openSaved('c1', 'r2');
    const tab = tabs.active!;
    tab.draft.url = 'edited';
    tabs.renamed('c1', 'r2', 'Ping');
    expect(tab.draft.name).toBe('Ping');
    expect(tab.draft.url).toBe('edited');
    expect(tab.dirty).toBe(true);
    tab.draft.url = draft('r2', '').url;
    expect(tab.dirty).toBe(false);
  });
});
