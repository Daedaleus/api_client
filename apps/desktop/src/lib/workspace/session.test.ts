import { describe, expect, it } from 'vitest';

import { FakeApi, changed, draft, memoryStorage, shop } from '../../test/fake-api';
import { RequestTab } from './request-tab.svelte';
import { SESSION_VERSION, isSession, sessionOf, withoutCredentials } from './session';
import { Workspace } from './workspace.svelte';

describe('remembered tabs', () => {
  it('keeps credentials only as variables', () => {
    const plain = withoutCredentials(
      draft('r1', 'Login', {
        auth: { kind: 'basic', username: 'alice', password: 's3cret' },
        headers: [
          { name: 'Authorization', value: 'Bearer abc', enabled: true },
          { name: 'Cookie', value: '{{session}}', enabled: true },
          { name: 'Accept', value: 'application/json', enabled: true },
        ],
      }),
    );
    expect(plain.auth).toEqual({ kind: 'basic', username: 'alice', password: '' });
    expect(plain.headers.map((header) => header.value)).toEqual([
      '',
      '{{session}}',
      'application/json',
    ]);
    const kinds = [
      { kind: 'bearer', token: '{{token}}' },
      { kind: 'apiKey', name: 'X-Key', value: 'k', placement: 'header' },
      { kind: 'inherit' },
      { kind: 'digest', username: 'bob', password: 'pw' },
      {
        kind: 'awsSigV4',
        accessKeyId: 'AKID',
        secretAccessKey: 's3cret',
        sessionToken: '{{awsToken}}',
        region: 'eu-central-1',
        service: 's3',
      },
    ] as const;
    expect(kinds.map((auth) => withoutCredentials(draft('x', 'X', { auth })).auth)).toEqual([
      { kind: 'bearer', token: '{{token}}' },
      { kind: 'apiKey', name: 'X-Key', value: '', placement: 'header' },
      { kind: 'inherit' },
      { kind: 'digest', username: 'bob', password: '' },
      {
        kind: 'awsSigV4',
        accessKeyId: 'AKID',
        secretAccessKey: '',
        sessionToken: '{{awsToken}}',
        region: 'eu-central-1',
        service: 's3',
      },
    ]);
  });

  it('remembers saved tabs by id, drafts only when needed, and the active tab', () => {
    const clean = new RequestTab(draft('r2', 'Health'), { collectionId: 'c1', requestId: 'r2' });
    const changed = new RequestTab(draft('r1', 'Login'), { collectionId: 'c1', requestId: 'r1' });
    changed.draft.url = 'https://example.com/v2';
    const scratch = RequestTab.scratch();
    scratch.draft.url = 'https://example.com/quick';
    const empty = RequestTab.scratch();
    const session = sessionOf([clean, empty, changed, scratch], changed.key);
    expect(session.version).toBe(SESSION_VERSION);
    expect(
      session.tabs.map((tab) => [tab.saved?.requestId ?? null, tab.draft?.url ?? null]),
    ).toEqual([
      ['r2', null],
      ['r1', 'https://example.com/v2'],
      [null, 'https://example.com/quick'],
    ]);
    expect(session.active).toBe(1);
    expect(isSession(session)).toBe(true);
    expect(isSession({ ...session, version: 99 })).toBe(false);
    expect(
      isSession({ version: 1, active: 0, tabs: [{ saved: { collectionId: 1 }, draft: null }] }),
    ).toBe(false);
    expect(isSession(null)).toBe(false);
  });

  it('brings the tabs back after the collections, and remembers changes', async () => {
    const api = new FakeApi();
    api.answers.push(changed(shop()));
    const stored = {
      version: SESSION_VERSION,
      active: 1,
      tabs: [
        { saved: { collectionId: 'c1', requestId: 'r2' }, draft: null },
        {
          saved: { collectionId: 'c1', requestId: 'r1' },
          draft: draft('r1', 'Login', { url: 'changed' }),
        },
        { saved: { collectionId: 'gone', requestId: 'x' }, draft: null },
        { saved: null, draft: draft('', '', { url: 'https://example.com/quick' }) },
      ],
    };
    const storage = memoryStorage({
      openCollections: JSON.stringify(['C:\\apis\\shop']),
      openTabs: JSON.stringify(stored),
    });
    const workspace = new Workspace(api, storage);
    // Nothing is written before the tabs are back.
    workspace.rememberTabs();
    expect(storage.data.get('openTabs')).toBe(JSON.stringify(stored));

    await workspace.restore();
    const tabs = workspace.tabs.tabs;
    expect(tabs.map((tab) => tab.saved?.requestId ?? tab.draft.url)).toEqual([
      'r2',
      'r1',
      'https://example.com/quick',
    ]);
    expect(workspace.tabs.active?.saved?.requestId).toBe('r1');
    expect(tabs[1]!.dirty).toBe(true);
    expect(tabs[1]!.draft.url).toBe('changed');

    workspace.tabs.close(tabs[0]!.key);
    workspace.rememberTabs();
    const remembered = JSON.parse(storage.data.get('openTabs')!) as typeof stored;
    expect(remembered.tabs).toHaveLength(2);
  });

  it('keeps the starting tab when nothing comes back', async () => {
    const api = new FakeApi();
    api.queryAnswers.push({ fail: new Error('gone') });
    const storage = memoryStorage({
      openTabs: JSON.stringify({
        version: SESSION_VERSION,
        active: 0,
        tabs: [{ saved: null, draft: null }],
      }),
    });
    const workspace = new Workspace(api, storage);
    await workspace.restore();
    expect(workspace.tabs.tabs).toHaveLength(1);
  });
});
