import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { Workspace } from '../../lib/workspace/workspace.svelte';
import { FakeApi, changed, shop } from '../../test/fake-api';
import RecipientsDialog from './RecipientsDialog.svelte';

async function setup(recipients: string[]) {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  api.queryAnswers.push({ type: 'recipients', recipients: { own: 'age1me', recipients } });
  await workspace.recipients.open('c1');
  render(RecipientsDialog, { recipients: workspace.recipients });
  return { api, workspace };
}

function clipboard(writeText: (text: string) => Promise<void>) {
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
}

describe('RecipientsDialog', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('shows and copies the own key', async () => {
    const writeText = vi.fn(() => Promise.resolve());
    clipboard(writeText);
    await setup(['age1me']);
    expect(screen.getByLabelText('Your public key')).toHaveValue('age1me');
    expect(screen.getByText('you')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Copy' }));
    expect(writeText).toHaveBeenCalledWith('age1me');
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });

  it('keeps working without clipboard access', async () => {
    clipboard(() => Promise.reject(new Error('denied')));
    await setup(['age1me']);
    await userEvent.click(screen.getByRole('button', { name: 'Copy' }));
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument();
  });

  it('adds with Enter or the button, removes and warns without the own key', async () => {
    await setup(['age1me']);
    const input = screen.getByLabelText('Public key (age1… or ssh-ed25519 …)');
    expect(screen.getByRole('button', { name: 'Add' })).toBeDisabled();
    await userEvent.type(input, 'age1team{Enter}');
    expect(input).toHaveValue('');
    await userEvent.type(input, 'age1ci');
    await userEvent.click(screen.getByRole('button', { name: 'Add' }));
    expect(screen.getByText('age1team')).toBeInTheDocument();
    expect(screen.getByText('age1ci')).toBeInTheDocument();

    await userEvent.click(screen.getAllByRole('button', { name: 'Remove recipient' })[0]!);
    expect(screen.getByRole('status')).toHaveTextContent('Your own key is missing');
  });

  it('saves, shows errors and closes with Escape', async () => {
    const { api, workspace } = await setup(['age1me']);
    api.answers.push({ fail: { key: 'error.secret_no_recipients', args: {} } });
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('nobody to encrypt');

    await userEvent.click(screen.getByLabelText('Your public key'));
    await userEvent.keyboard('{Escape}');
    expect(workspace.recipients.editing).toBeNull();

    api.queryAnswers.push({ type: 'recipients', recipients: { own: 'age1me', recipients: [] } });
    await workspace.recipients.open('c1');
    api.answers.push(changed(shop()));
    await userEvent.click(await screen.findByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(workspace.recipients.editing).toBeNull());

    api.queryAnswers.push({ type: 'recipients', recipients: { own: 'age1me', recipients: [] } });
    await workspace.recipients.open('c1');
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(workspace.recipients.editing).toBeNull();
  });
});
