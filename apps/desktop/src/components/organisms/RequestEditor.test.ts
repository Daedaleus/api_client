import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { RequestTab } from '../../lib/workspace/request-tab.svelte';
import { draft } from '../../test/fake-api';
import RequestEditor from './RequestEditor.svelte';

function setup(
  tab = new RequestTab(draft('r1', 'Login'), { collectionId: 'c1', requestId: 'r1' }),
  variables: ReadonlyMap<string, string> = new Map(),
) {
  const onsave = vi.fn();
  render(RequestEditor, { tab, onsend: vi.fn(), onsave, variables });
  return { tab, onsave };
}

describe('RequestEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits query parameters and path variables in step with the URL', async () => {
    const { tab } = setup();
    tab.draft.url = '{{baseUrl}}/users/:id?page=2';
    await screen.findByText(':id');
    expect(screen.getByRole('tab', { name: /Params/ })).toHaveTextContent('2');
    const query = screen.getByRole('region', { name: 'Query parameters' });
    expect(within(query).getByDisplayValue('page')).toBeInTheDocument();

    // A new row goes into the URL; switching it off moves it out.
    await userEvent.click(within(query).getByRole('button', { name: 'Add parameter' }));
    const names = within(query).getAllByLabelText('Parameter name');
    await userEvent.type(names[1]!, 'q');
    await userEvent.type(within(query).getAllByLabelText('Value')[1]!, 'tea');
    expect(tab.draft.url).toBe('{{baseUrl}}/users/:id?page=2&q=tea');
    await userEvent.click(within(query).getAllByRole('checkbox', { name: 'Send' })[0]!);
    expect(tab.draft.url).toBe('{{baseUrl}}/users/:id?q=tea');
    expect(tab.draft.disabledQuery).toEqual([{ name: 'page', value: '2' }]);

    // Typed into the URL: the table follows.
    tab.draft.url = '{{baseUrl}}/users/:id?q=coffee';
    await vi.waitFor(() => expect(within(query).getByDisplayValue('coffee')).toBeInTheDocument());

    // Path variable values are kept and previewed.
    await userEvent.type(screen.getByLabelText('Value of :id'), '42');
    expect(tab.draft.pathParams).toEqual([{ name: 'id', value: '42' }]);
    expect(screen.getByText(/users\/42\?q=coffee/)).toBeInTheDocument();
  });

  it('edits headers: add, name, value, switch off, remove', async () => {
    const { tab } = setup();
    await userEvent.click(screen.getByRole('tab', { name: /Headers/ }));
    expect(screen.getByText('No headers.')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Add header' }));
    await userEvent.type(screen.getByLabelText('Header name'), 'X-Trace');
    await userEvent.type(screen.getByLabelText('Value'), 'abc');
    expect(tab.draft.headers).toEqual([{ name: 'X-Trace', value: 'abc', enabled: true }]);
    expect(screen.getByRole('tab', { name: /Headers/ })).toHaveTextContent('1');
    expect(tab.dirty).toBe(true);

    await userEvent.click(screen.getByRole('checkbox', { name: 'Send' }));
    expect(tab.draft.headers[0]?.enabled).toBe(false);
    expect(screen.getByRole('tab', { name: /Headers/ })).not.toHaveTextContent('1');

    await userEvent.click(screen.getByRole('button', { name: 'Remove header' }));
    expect(tab.draft.headers).toEqual([]);
  });

  it('edits the body and points out invalid JSON', async () => {
    const { tab } = setup();
    await userEvent.click(screen.getByRole('tab', { name: 'Body' }));
    expect(screen.getByText('This request sends no body.')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('radio', { name: 'JSON' }));
    const text = screen.getByLabelText('Body');
    await userEvent.type(text, '{{"a": 1');
    expect(screen.getByRole('status')).toHaveTextContent('Not valid JSON');
    expect(text).toHaveAttribute('aria-invalid', 'true');

    await userEvent.type(text, '}');
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(tab.draft.body).toEqual({ kind: 'json', content: '{"a": 1}' });
    expect(screen.getByRole('tab', { name: /Body/ })).toHaveTextContent('JSON');

    await userEvent.click(screen.getByRole('radio', { name: 'Text' }));
    expect(tab.draft.body).toEqual({ kind: 'text', content: '{"a": 1}' });
    await userEvent.click(screen.getByRole('radio', { name: 'None' }));
    expect(tab.draft.body).toEqual({ kind: 'none' });
  });

  it('saves with the button', async () => {
    const { onsave } = setup();
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(onsave).toHaveBeenCalled();
  });

  it('cannot save read-only requests', () => {
    setup(
      new RequestTab(draft('r1', 'Old', { readOnly: true }), {
        collectionId: 'c1',
        requestId: 'r1',
      }),
    );
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(screen.getByText(/newer version/)).toBeInTheDocument();
  });

  it('previews the URL with variables and warns about unknown ones', async () => {
    const tab = new RequestTab(draft('r1', 'Login', { url: '{{baseUrl}}/{{path}}' }), {
      collectionId: 'c1',
      requestId: 'r1',
    });
    setup(tab, new Map([['baseUrl', 'http://localhost:8080']]));
    expect(screen.getByText('Sent to: http://localhost:8080/{{path}}')).toBeInTheDocument();
    expect(screen.getByText('Unknown variables: path')).toBeInTheDocument();

    await userEvent.clear(screen.getByLabelText('URL'));
    await userEvent.type(screen.getByLabelText('URL'), 'https://plain.test');
    expect(screen.queryByText(/Sent to/)).not.toBeInTheDocument();
  });

  it('lists unknown variables when nothing defines them', () => {
    setup(new RequestTab(draft('r1', 'Login', { url: '{{baseUrl}}/x' }), null));
    expect(screen.getByText('Unknown variables: baseUrl')).toBeInTheDocument();
  });

  it('edits the request auth', async () => {
    const { tab } = setup();
    await userEvent.click(screen.getByRole('tab', { name: /Auth/ }));
    expect(screen.getByText(/Uses the auth of the folder/)).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByLabelText('Type'), 'Basic');
    await userEvent.type(screen.getByLabelText('Username'), 'alice');
    expect(tab.draft.auth).toEqual({ kind: 'basic', username: 'alice', password: '' });
    expect(screen.getByRole('tab', { name: /Auth/ })).toHaveTextContent('Basic');
  });

  it('switches the certificate check and warns when it is off', async () => {
    const tab = new RequestTab(draft('r1', 'Login'), { collectionId: 'c1', requestId: 'r1' });
    render(RequestEditor, {
      tab,
      onsend: vi.fn(),
      onsave: vi.fn(),
      checksCertificates: false,
    });
    expect(screen.getByRole('status')).toHaveTextContent('TLS certificates are not checked');
    await userEvent.click(screen.getByRole('tab', { name: 'Settings' }));
    await userEvent.selectOptions(screen.getByLabelText('Check TLS certificate'), 'off');
    expect(tab.draft.certificateCheck).toBe('off');
    expect(screen.getByText(/connections are then not protected/)).toBeInTheDocument();
  });

  it('copies the request as cURL and shows notices', async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const tab = new RequestTab(draft('r1', 'Login'), null);
    tab.notice = { key: 'curl.ignored', args: { options: '-L' } };
    const oncopycurl = vi
      .fn<() => Promise<string | null>>()
      .mockResolvedValueOnce(null)
      .mockResolvedValue("curl 'https://x'");
    render(RequestEditor, { tab, onsend: vi.fn(), onsave: vi.fn(), oncopycurl });
    expect(screen.getByText('Not taken over: -L')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Copy as cURL' }));
    expect(writeText).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Copy as cURL' }));
    expect(writeText).toHaveBeenCalledWith("curl 'https://x'");
    expect(screen.getByRole('button', { name: 'cURL copied' })).toBeInTheDocument();

    writeText.mockRejectedValueOnce(new Error('denied'));
    await userEvent.click(screen.getByRole('button', { name: 'cURL copied' }));
  });

  it('edits the request variables', async () => {
    const { tab } = setup();
    await userEvent.click(screen.getByRole('tab', { name: /Variables/ }));
    expect(screen.getByText('No own variables.')).toBeInTheDocument();
    expect(screen.getByText(/override folders, environment and collection/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add variable' }));
    await userEvent.type(screen.getByLabelText('Variable name'), 'path');
    expect(tab.draft.variables).toEqual([{ name: 'path', value: '', enabled: true }]);
    expect(screen.getByRole('tab', { name: /Variables/ })).toHaveTextContent('1');
  });
});
