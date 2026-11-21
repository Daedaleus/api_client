import { describe, expect, it } from 'vitest';

import { FakeApi, OK, changed, draft, memoryStorage, shop } from '../../test/fake-api';
import type { CollectionView, VariableDraft } from '../types';
import { Workspace } from './workspace.svelte';

const row = (name: string, value: string): VariableDraft => ({
  name,
  value,
  enabled: true,
  secret: false,
});

/** Shop with variables on the collection and on the folder Auth. */
function layered(): CollectionView {
  const collection = shop();
  collection.variables = [row('host', 'collection'), row('a', '1')];
  const auth = collection.items[0]!;
  if (auth.kind === 'folder') auth.variables = [row('host', 'folder')];
  return collection;
}

function setup(collection = layered()) {
  const api = new FakeApi();
  const workspace = new Workspace(api, memoryStorage());
  workspace.collections.apply(collection);
  return { api, workspace };
}

describe('variables and auth in the workspace', () => {
  it('previews with collection, environment, folders and the request, stronger winning', async () => {
    const { api, workspace } = setup();
    api.queryAnswers.push({
      type: 'request',
      request: draft('r1', 'Login', { variables: [row('a', 'own')] }),
    });
    await workspace.openRequest('c1', 'r1');
    const tab = workspace.tabs.active!;
    workspace.environments.choose('c1', 'e1');
    expect(Object.fromEntries(workspace.variablesFor(tab))).toMatchObject({
      host: 'folder',
      a: 'own',
      baseUrl: 'http://localhost:8080',
    });

    await workspace.send(tab);
    expect(api.commands.at(-1)).toMatchObject({
      type: 'sendRequest',
      variables: [row('a', 'own')],
      auth: { kind: 'inherit' },
      context: { collectionId: 'c1', requestId: 'r1', environmentId: 'e1' },
    });
    workspace.environments.choose('c1', null);
    await workspace.send(tab);
    expect(api.commands.at(-1)).toMatchObject({ context: { environmentId: null } });
    api.emit(1, OK);
  });

  it('scratch tabs only use their own variables', async () => {
    const { api, workspace } = setup();
    const tab = workspace.tabs.active!;
    tab.draft.variables = [row('host', 'own')];
    expect(Object.fromEntries(workspace.variablesFor(tab))).toEqual({ host: 'own' });
    await workspace.send(tab);
    expect(api.commands.at(-1)).toMatchObject({ context: null });
  });

  it('edits the variables of the collection and of folders', async () => {
    const { api, workspace } = setup();
    const state = workspace.defaults;
    state.open('c1', 'missing');
    state.open('other', 'c1');
    expect(state.editing).toBeNull();

    state.open('c1', 'c1');
    expect(state.editing).toMatchObject({ name: 'Shop', isCollection: true });
    state.open('c1', 'f1');
    expect(state.editing).toMatchObject({
      name: 'Auth',
      isCollection: false,
      variables: [row('host', 'folder')],
    });
    state.editing!.variables.push(row('v', '2'));
    state.editing!.auth = { kind: 'bearer', token: '{{token}}' };
    expect(workspace.collections.find('c1')?.items[0]).toMatchObject({
      variables: [row('host', 'folder')],
    });

    api.answers.push({ fail: { key: 'error.store_read_only', args: { path: 'x' } } });
    await state.save();
    expect(state.editing).not.toBeNull();
    expect(state.error?.key).toBe('error.store_read_only');

    api.answers.push(changed({ ...layered(), name: 'Shop 2' }));
    await state.save();
    expect(api.commands.at(-1)).toEqual({
      type: 'saveDefaults',
      collectionId: 'c1',
      itemId: 'f1',
      variables: [row('host', 'folder'), row('v', '2')],
      auth: { kind: 'bearer', token: '{{token}}' },
      certificateCheck: 'inherit',
    });
    expect(state.editing).toBeNull();
    expect(workspace.collections.find('c1')?.name).toBe('Shop 2');
    await state.save();
    expect(api.commands).toHaveLength(2);

    state.open('c1', 'c1');
    api.answers.push({ type: 'runStarted', runId: 1 });
    await state.save();
    expect(state.error?.args.detail).toBe('unexpected answer runStarted');
    state.close();
    expect(state.editing).toBeNull();
  });

  it('works out whether certificates are checked, as when sending', async () => {
    const collection = layered();
    const auth = collection.items[0]!;
    if (auth.kind === 'folder') auth.certificateCheck = 'off';
    const { workspace } = setup(collection);
    const scratch = workspace.tabs.active!;
    expect(workspace.checksCertificates(scratch)).toBe(true);
    scratch.draft.certificateCheck = 'off';
    expect(workspace.checksCertificates(scratch)).toBe(false);

    await workspace.openRequest('c1', 'r1');
    const login = workspace.tabs.active!;
    expect(workspace.checksCertificates(login)).toBe(false);
    login.draft.certificateCheck = 'on';
    expect(workspace.checksCertificates(login)).toBe(true);

    await workspace.openRequest('c1', 'r2');
    expect(workspace.checksCertificates(workspace.tabs.active!)).toBe(true);
  });
});
