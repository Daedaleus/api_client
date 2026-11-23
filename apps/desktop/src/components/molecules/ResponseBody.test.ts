import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { BodyView } from '../../lib/types';
import ResponseBody from './ResponseBody.svelte';

const JSON_BODY: BodyView = {
  text: JSON.stringify({ items: [{ id: 1 }, { id: 2 }], name: 'shop' }, null, 2),
  pretty: true,
  truncated: false,
  binary: false,
};

function clipboard() {
  const writeText = vi.fn(() => Promise.resolve());
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
  return writeText;
}

describe('ResponseBody', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('filters JSON and copies the result', async () => {
    const writeText = clipboard();
    render(ResponseBody, { body: JSON_BODY, sizeBytes: 50 });
    const filter = screen.getByRole('searchbox', { name: 'Filter JSON' });
    await userEvent.type(filter, '$.items[[*].id');
    await userEvent.click(screen.getByRole('button', { name: 'Copy result' }));
    expect(writeText).toHaveBeenCalledWith('[\n  1,\n  2\n]');
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();

    await userEvent.clear(filter);
    await userEvent.type(filter, '$.name');
    // Still "Copied" from before (for two seconds).
    await userEvent.click(screen.getByRole('button', { name: /Copy result|Copied/ }));
    expect(writeText).toHaveBeenLastCalledWith('"shop"');

    await userEvent.clear(filter);
    await userEvent.type(filter, '$.missing');
    expect(screen.getByRole('status')).toHaveTextContent('Nothing found.');
    await userEvent.click(screen.getByRole('button', { name: /Copy body|Copied/ }));
    expect(writeText).toHaveBeenLastCalledWith(JSON_BODY.text);

    await userEvent.clear(filter);
    await userEvent.type(filter, 'items');
    expect(screen.getByRole('status')).toHaveTextContent('A JSONPath starts with $.');
  });

  it('says when a shortened body cannot be filtered', async () => {
    render(ResponseBody, {
      body: { text: '{"items": [1, 2', pretty: true, truncated: true, binary: false },
      sizeBytes: 9000,
    });
    await userEvent.type(screen.getByRole('searchbox', { name: 'Filter JSON' }), '$.items');
    expect(screen.getByRole('status')).toHaveTextContent('no complete JSON');
  });

  it('saves any body, also binary ones; copies only text', async () => {
    const writeText = clipboard();
    writeText.mockRejectedValueOnce(new Error('denied'));
    const onsave = vi.fn();
    const { unmount } = render(ResponseBody, {
      body: { text: '', pretty: false, truncated: false, binary: true },
      sizeBytes: 2048,
      onsave,
    });
    expect(screen.queryByRole('searchbox')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Copy body' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Save …' }));
    expect(onsave).toHaveBeenCalledOnce();
    unmount();

    render(ResponseBody, {
      body: { text: 'plain', pretty: false, truncated: false, binary: false },
      sizeBytes: 5,
    });
    expect(screen.queryByRole('button', { name: 'Save …' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Copy body' }));
    expect(screen.getByRole('button', { name: 'Copy body' })).toBeInTheDocument();
  });
});
