import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { AuthDraft } from '../../lib/types';
import AuthEditor, { authOf, hasPlainCredential } from './AuthEditor.svelte';

describe('AuthEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('switches kinds, keeping what fits', () => {
    const basic: AuthDraft = { kind: 'basic', username: 'a', password: 'b' };
    expect(authOf('basic', basic)).toBe(basic);
    expect(authOf('bearer', basic)).toEqual({ kind: 'bearer', token: '' });
    expect(authOf('apiKey', basic)).toEqual({
      kind: 'apiKey',
      name: '',
      value: '',
      placement: 'header',
    });
    const key: AuthDraft = { kind: 'apiKey', name: 'k', value: 'v', placement: 'query' };
    expect(authOf('apiKey', key)).toBe(key);
    const bearer: AuthDraft = { kind: 'bearer', token: 't' };
    expect(authOf('bearer', bearer)).toBe(bearer);
    expect(authOf('basic', bearer)).toEqual({ kind: 'basic', username: '', password: '' });
    expect(authOf('none', basic)).toEqual({ kind: 'none' });
    expect(authOf('inherit', basic)).toEqual({ kind: 'inherit' });
  });

  it('flags credentials typed in plain text', () => {
    expect(hasPlainCredential({ kind: 'bearer', token: 's3cr3t' })).toBe(true);
    expect(hasPlainCredential({ kind: 'bearer', token: '{{token}}' })).toBe(false);
    expect(hasPlainCredential({ kind: 'basic', username: 'alice', password: '' })).toBe(false);
    expect(hasPlainCredential({ kind: 'apiKey', name: 'k', value: 'v', placement: 'header' })).toBe(
      true,
    );
    expect(hasPlainCredential({ kind: 'none' })).toBe(false);
  });

  it('edits an API key and where it goes', async () => {
    const auth = $state<{ value: AuthDraft }>({ value: { kind: 'none' } });
    render(AuthEditor, {
      get auth() {
        return auth.value;
      },
      set auth(next) {
        auth.value = next;
      },
    });
    await userEvent.selectOptions(screen.getByLabelText('Type'), 'API key');
    await userEvent.type(screen.getByLabelText('Name'), 'X-Key');
    await userEvent.type(screen.getByLabelText('Value'), '{{{{key}}');
    await userEvent.selectOptions(screen.getByLabelText('Send as'), 'Query parameter');
    expect(auth.value).toEqual({
      kind: 'apiKey',
      name: 'X-Key',
      value: '{{key}}',
      placement: 'query',
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('edits OAuth 2 client credentials', async () => {
    const auth = $state<{ value: AuthDraft }>({ value: { kind: 'bearer', token: '' } });
    render(AuthEditor, {
      get auth() {
        return auth.value;
      },
      set auth(next) {
        auth.value = next;
      },
    });
    await userEvent.selectOptions(screen.getByLabelText('Type'), 'OAuth 2');
    expect(screen.queryByLabelText('Authorization URL')).not.toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Token URL'), 'https://auth.test/token');
    await userEvent.type(screen.getByLabelText('Client ID'), 'app');
    await userEvent.type(screen.getByLabelText('Client secret'), 'pw');
    await userEvent.type(screen.getByLabelText('Scope'), 'read');
    expect(auth.value).toEqual({
      kind: 'oauth2',
      grant: 'clientCredentials',
      tokenUrl: 'https://auth.test/token',
      authUrl: '',
      redirectUrl: '',
      clientId: 'app',
      clientSecret: 'pw',
      scope: 'read',
    });
    expect(screen.getByRole('status')).toHaveTextContent('plain text');
    expect(screen.getByText(/never saved/)).toBeInTheDocument();
    expect(authOf('oauth2', auth.value)).toBe(auth.value);
  });

  it('edits the authorization code sign-in', async () => {
    const auth = $state<{ value: AuthDraft }>({ value: { kind: 'none' } });
    render(AuthEditor, {
      get auth() {
        return auth.value;
      },
      set auth(next) {
        auth.value = next;
      },
    });
    await userEvent.selectOptions(screen.getByLabelText('Type'), 'OAuth 2');
    await userEvent.selectOptions(
      screen.getByLabelText('Grant type'),
      'Authorization code (sign in in the browser)',
    );
    await userEvent.type(screen.getByLabelText('Authorization URL'), 'https://auth.test/authorize');
    const redirect = screen.getByLabelText('Redirect URL');
    expect(redirect).toHaveAttribute('placeholder', 'empty: http://127.0.0.1 with a free port');
    await userEvent.type(redirect, 'http://127.0.0.1:8080/cb');
    expect(auth.value).toMatchObject({
      grant: 'authorizationCode',
      authUrl: 'https://auth.test/authorize',
      redirectUrl: 'http://127.0.0.1:8080/cb',
    });
    expect(screen.getByText(/browser opens/)).toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });
});
