import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { Workspace } from '../../lib/workspace/workspace.svelte';
import { FakeApi, changed, shop } from '../../test/fake-api';
import DefaultsDialog from './DefaultsDialog.svelte';

function setup(itemId: string) {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  workspace.collections.apply(shop());
  workspace.defaults.open('c1', itemId);
  render(DefaultsDialog, { defaults: workspace.defaults });
  return { api, workspace };
}

describe('DefaultsDialog', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits and saves the collection variables', async () => {
    const { api, workspace } = setup('c1');
    expect(screen.getByRole('heading')).toHaveTextContent('“Shop”: variables and auth');
    expect(screen.getByText(/every request of the collection/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add variable' }));
    await userEvent.type(screen.getByLabelText('Variable name'), 'host');

    api.answers.push({ fail: { key: 'error.store_read_only', args: { path: 'x' } } });
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toBeInTheDocument();

    api.answers.push(changed(shop()));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(workspace.defaults.editing).toBeNull());
    expect(api.commands.at(-1)).toMatchObject({ itemId: 'c1', variables: [{ name: 'host' }] });
  });

  it('explains folder variables and cancels with Escape or the button', async () => {
    const { workspace } = setup('f1');
    expect(screen.getByText(/every request in this folder/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add variable' }));
    await userEvent.keyboard('{Escape}');
    expect(workspace.defaults.editing).toBeNull();

    workspace.defaults.open('c1', 'f1');
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(workspace.defaults.editing).toBeNull();
  });

  it('edits the auth; the collection has nothing to inherit from', async () => {
    const { api, workspace } = setup('c1');
    await userEvent.click(screen.getByRole('tab', { name: 'Auth' }));
    const kind = screen.getByLabelText('Type');
    expect([...(kind as HTMLSelectElement).options].map((option) => option.text)).toEqual([
      'None',
      'Basic',
      'Bearer token',
      'API key',
    ]);
    await userEvent.selectOptions(kind, 'Bearer token');
    await userEvent.type(screen.getByLabelText('Token'), 'abc');
    expect(screen.getByRole('status')).toHaveTextContent('plain text');
    api.answers.push(changed(shop()));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(workspace.defaults.editing).toBeNull());
    expect(api.commands.at(-1)).toMatchObject({ auth: { kind: 'bearer', token: 'abc' } });
  });
});
