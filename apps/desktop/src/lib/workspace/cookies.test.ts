import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import CookiesDialog from '../../components/organisms/CookiesDialog.svelte';
import DialogHost from '../../components/organisms/DialogHost.svelte';
import { FakeApi } from '../../test/fake-api';
import { i18n } from '../i18n/i18n.svelte';
import type { CookieView } from '../types';
import { CookiesState, keyOf } from './cookies.svelte';
import { Dialogs } from './dialogs.svelte';

const session: CookieView = {
  name: 'session',
  value: 's3cret',
  domain: 'x.example',
  hostOnly: true,
  path: '/',
  expires: null,
  secure: true,
  httpOnly: true,
};
const lang: CookieView = {
  ...session,
  name: 'lang',
  value: 'de',
  hostOnly: false,
  expires: 1_800_000_000,
};

function setup() {
  const api = new FakeApi();
  const dialogs = new Dialogs();
  const cookies = new CookiesState(api, dialogs);
  render(CookiesDialog, { cookies });
  render(DialogHost, { dialogs });
  return { api, cookies };
}

describe('cookie manager', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('lists the cookies of a collection', async () => {
    const { api, cookies } = setup();
    api.queryAnswers.push({ type: 'cookies', cookies: [session, lang] });
    await cookies.open('c1', 'Shop');
    expect(api.queries.at(-1)).toEqual({ type: 'cookies', collectionId: 'c1' });
    expect(await screen.findByRole('dialog')).toHaveTextContent('Cookies – Shop');
    const rows = screen.getAllByRole('row');
    expect(rows).toHaveLength(3);
    expect(rows[1]).toHaveTextContent('sessions3cretx.example/Session');
    // Domain cookies show the leading dot, dates in the user's language.
    expect(rows[2]).toHaveTextContent('.x.example');
    expect(rows[2]).toHaveTextContent(new Date(1_800_000_000_000).toLocaleString('en'));
  });

  it('shows an empty jar and errors', async () => {
    const { api, cookies } = setup();
    api.queryAnswers.push({ type: 'cookies', cookies: [] });
    await cookies.open('c1', 'Shop');
    expect(await screen.findByText('No cookies yet.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete all' })).toBeDisabled();

    api.queryAnswers.push({ fail: { key: 'error.unknown_collection', args: {} } });
    await cookies.open('c9', 'Gone');
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    api.queryAnswers.push({ type: 'history', entries: [] });
    await cookies.open('c1', 'Shop');
    expect(cookies.error?.args.detail).toBe('unexpected answer history');
  });

  it('adds and edits cookies; a refused one keeps the form open', async () => {
    const { api, cookies } = setup();
    api.queryAnswers.push({ type: 'cookies', cookies: [session] });
    await cookies.open('c1', 'Shop');
    await userEvent.click(await screen.findByRole('button', { name: 'Add cookie' }));
    expect(screen.getByRole('heading', { name: 'New cookie' })).toBeInTheDocument();
    expect(cookies.editing?.draft.domain).toBe('x.example');
    await userEvent.type(screen.getByLabelText('Name'), 'lang');
    await userEvent.type(screen.getByLabelText('Value'), 'de');
    await userEvent.click(screen.getByLabelText('Also for subdomains'));
    await userEvent.click(screen.getByLabelText('Only over HTTPS (Secure)'));

    api.answers.push({ fail: { key: 'error.cookie_name', args: {} } });
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The name must not be empty');
    expect(cookies.editing).not.toBeNull();

    api.answers.push({ type: 'cookiesChanged', cookies: [session, lang] });
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(api.commands.at(-1)).toEqual({
      type: 'saveCookie',
      collectionId: 'c1',
      original: null,
      cookie: {
        ...session,
        name: 'lang',
        value: 'de',
        hostOnly: false,
        secure: true,
        httpOnly: false,
      },
    });
    expect(cookies.editing).toBeNull();
    expect(screen.getAllByRole('row')).toHaveLength(3);

    await userEvent.click(screen.getByRole('button', { name: 'Edit “lang”' }));
    expect(screen.getByRole('heading', { name: 'Edit cookie' })).toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText('Value'));
    await userEvent.type(screen.getByLabelText('Value'), 'en');
    api.answers.push({ type: 'cookiesChanged', cookies: [session, { ...lang, value: 'en' }] });
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(api.commands.at(-1)).toMatchObject({
      type: 'saveCookie',
      original: keyOf(lang),
      cookie: { name: 'lang', value: 'en' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Edit “lang”' }));
    await userEvent.keyboard('{Escape}');
    expect(cookies.editing).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Add cookie' }));
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(cookies.editing).toBeNull();
  });

  it('deletes one cookie and clears all after asking', async () => {
    const { api, cookies } = setup();
    api.queryAnswers.push({ type: 'cookies', cookies: [session, lang] });
    await cookies.open('c1', 'Shop');
    api.answers.push({ type: 'cookiesChanged', cookies: [lang] });
    await userEvent.click(await screen.findByRole('button', { name: 'Delete “session”' }));
    expect(api.commands.at(-1)).toEqual({
      type: 'deleteCookie',
      collectionId: 'c1',
      cookie: keyOf(session),
    });
    expect(screen.getAllByRole('row')).toHaveLength(2);

    await userEvent.click(screen.getByRole('button', { name: 'Delete all' }));
    const ask = screen.getAllByRole('dialog').at(-1)!;
    expect(ask).toHaveTextContent('Delete all cookies?');
    await userEvent.click(within(ask).getByRole('button', { name: 'Cancel' }));
    expect(api.commands).toHaveLength(1);

    api.answers.push({ type: 'cookiesChanged', cookies: [] });
    await userEvent.click(screen.getByRole('button', { name: 'Delete all' }));
    await userEvent.click(
      within(screen.getAllByRole('dialog').at(-1)!).getByRole('button', { name: 'OK' }),
    );
    await screen.findByText('No cookies yet.');
    expect(api.commands.at(-1)).toEqual({ type: 'clearCookies', collectionId: 'c1' });

    api.answers.push({ type: 'collectionClosed', collectionId: 'c1' });
    await cookies.remove(lang);
    expect(cookies.error?.args.detail).toBe('unexpected answer collectionClosed');
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(cookies.shown).toBeNull();
    await cookies.save();
    expect(api.commands).toHaveLength(3);
  });
});
