import { describe, expect, it } from 'vitest';

import { FakeApi, changed, memoryStorage, shop } from '../../test/fake-api';
import { Workspace } from './workspace.svelte';

const OWN = 'age1me';

function setup(recipients: string[] = []) {
  const api = new FakeApi();
  const workspace = new Workspace(api, memoryStorage());
  workspace.collections.apply(shop());
  api.queryAnswers.push({ type: 'recipients', recipients: { own: OWN, recipients } });
  return { api, state: workspace.recipients, workspace };
}

describe('RecipientsState', () => {
  it('starts with the own key when there is no list yet', async () => {
    const { api, state } = setup();
    await state.open('c1');
    expect(api.queries.at(-1)).toEqual({ type: 'recipients', collectionId: 'c1' });
    expect(state.editing).toEqual({ collectionId: 'c1', own: OWN, list: [OWN] });
    expect(state.withoutOwnKey).toBe(false);
  });

  it('adds keys once, removes them and warns without the own key', async () => {
    const { state } = setup(['age1me', 'age1team']);
    await state.open('c1');
    expect(state.add('  ')).toBe(false);
    expect(state.add(' age1team ')).toBe(false);
    expect(state.add(' ssh-ed25519 AAAA bob ')).toBe(true);
    expect(state.editing?.list).toEqual(['age1me', 'age1team', 'ssh-ed25519 AAAA bob']);
    state.remove(0);
    expect(state.withoutOwnKey).toBe(true);
  });

  it('saves the list and keeps the dialog open on failure', async () => {
    const { api, state, workspace } = setup([OWN]);
    await state.open('c1');
    state.add('age1team');

    api.answers.push({
      fail: { key: 'error.secret_not_reencrypted', args: { environment: 'Dev' } },
    });
    await state.save();
    expect(state.editing).not.toBeNull();
    expect(state.error?.key).toBe('error.secret_not_reencrypted');
    expect(state.saving).toBe(false);

    api.answers.push(changed({ ...shop(), name: 'Shop 2' }));
    await state.save();
    expect(api.commands.at(-1)).toEqual({
      type: 'setRecipients',
      collectionId: 'c1',
      recipients: [OWN, 'age1team'],
    });
    expect(state.editing).toBeNull();
    expect(workspace.collections.find('c1')?.name).toBe('Shop 2');
    await state.save();
    expect(api.commands).toHaveLength(2);
  });

  it('reports failures to load and unexpected answers', async () => {
    const api = new FakeApi();
    const workspace = new Workspace(api, null);
    api.queryAnswers.push({ fail: { key: 'error.secret_key', args: { detail: 'x' } } });
    await workspace.recipients.open('c1');
    expect(workspace.recipients.editing).toBeNull();
    expect(workspace.collections.error?.key).toBe('error.secret_key');
    expect(workspace.recipients.add('age1x')).toBe(false);
    expect(workspace.recipients.withoutOwnKey).toBe(false);

    api.queryAnswers.push({ type: 'request', request: shop() as never });
    await workspace.recipients.open('c1');
    expect(workspace.collections.error?.args.detail).toBe('unexpected answer request');

    api.queryAnswers.push({ type: 'recipients', recipients: { own: OWN, recipients: [] } });
    await workspace.recipients.open('c1');
    api.answers.push({ type: 'runStarted', runId: 1 });
    await workspace.recipients.save();
    expect(workspace.recipients.error?.args.detail).toBe('unexpected answer runStarted');
  });
});
