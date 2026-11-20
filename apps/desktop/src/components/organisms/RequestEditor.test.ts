import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { RequestTab } from '../../lib/workspace/request-tab.svelte';
import { draft } from '../../test/fake-api';
import RequestEditor from './RequestEditor.svelte';

function setup(
  tab = new RequestTab(draft('r1', 'Login'), { collectionId: 'c1', requestId: 'r1' }),
) {
  const onsave = vi.fn();
  render(RequestEditor, { tab, onsend: vi.fn(), onsave });
  return { tab, onsave };
}

describe('RequestEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits headers: add, name, value, switch off, remove', async () => {
    const { tab } = setup();
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
});
