import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { SettingsState } from '../../lib/workspace/settings.svelte';
import { FakeApi } from '../../test/fake-api';
import SettingsDialog from './SettingsDialog.svelte';

async function setup() {
  const api = new FakeApi();
  const settings = new SettingsState(api);
  api.queryAnswers.push({
    type: 'settings',
    settings: { proxy: { mode: 'system' }, caCertificates: [] },
    problem: null,
  });
  render(SettingsDialog, { settings });
  await settings.open();
  return { api, settings };
}

describe('SettingsDialog', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('sets a proxy and CA certificates and saves them', async () => {
    const { api, settings } = await setup();
    const dialog = await screen.findByRole('dialog', { name: 'Settings' });
    await userEvent.selectOptions(within(dialog).getByLabelText('Proxy'), 'manual');
    const url = within(dialog).getByLabelText('Proxy URL');
    await userEvent.type(url, 'http://proxy:8080');
    expect(within(dialog).getByLabelText('Without proxy for')).toHaveValue('localhost');

    api.file = 'C:\\certs\\corp.pem';
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add certificate …' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add certificate …' }));
    expect(within(dialog).getAllByText('C:\\certs\\corp.pem')).toHaveLength(1);
    expect(api.picks.at(-1)?.options?.filters?.[0]?.extensions).toContain('pem');

    // Refused by the backend: the dialog stays open with the reason.
    api.answers.push({ fail: { key: 'error.settings', args: { detail: 'bad proxy' } } });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('bad proxy');

    api.answers.push({ type: 'settingsSaved' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(settings.editing).toBeNull());
    expect(api.commands.at(-1)).toEqual({
      type: 'saveSettings',
      settings: {
        proxy: { mode: 'manual', url: 'http://proxy:8080', noProxy: 'localhost' },
        caCertificates: ['C:\\certs\\corp.pem'],
      },
    });
  });

  it('removes certificates, switches the language and cancels', async () => {
    const { settings } = await setup();
    settings.editing!.caCertificates.push('a.pem');
    const dialog = await screen.findByRole('dialog', { name: 'Settings' });
    await userEvent.click(await within(dialog).findByRole('button', { name: 'Remove “a.pem”' }));
    expect(settings.editing!.caCertificates).toEqual([]);
    expect(within(dialog).getByText(/system's certificates apply/)).toBeInTheDocument();

    await userEvent.selectOptions(within(dialog).getByRole('combobox', { name: 'Language' }), 'de');
    expect(i18n.locale).toBe('de');
    expect(screen.getByRole('dialog', { name: 'Einstellungen' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(settings.editing).toBeNull();
  });

  it('opens with the defaults and the reason if the settings cannot be read', async () => {
    const api = new FakeApi();
    const settings = new SettingsState(api);
    api.queryAnswers.push({ fail: { key: 'error.settings', args: { detail: 'damaged' } } });
    await settings.open();
    expect(settings.editing).toEqual({ proxy: { mode: 'system' }, caCertificates: [] });
    expect(settings.error?.args['detail']).toBe('damaged');

    api.queryAnswers.push({
      type: 'settings',
      settings: { proxy: { mode: 'none' }, caCertificates: [] },
      problem: { key: 'error.settings', args: { detail: 'at start' } },
    });
    expect((await settings.startupProblem())?.args['detail']).toBe('at start');
    api.queryAnswers.push({ fail: new Error('gone') });
    expect(await settings.startupProblem()).not.toBeNull();
  });
});
