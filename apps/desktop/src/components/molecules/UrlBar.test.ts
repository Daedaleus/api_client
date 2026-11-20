import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import UrlBar from './UrlBar.svelte';

describe('UrlBar', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('sends on click and on Enter', async () => {
    const onsend = vi.fn();
    render(UrlBar, { method: 'GET', url: '', sending: false, error: null, onsend });

    await userEvent.type(screen.getByLabelText('URL'), 'example.com{Enter}');
    await userEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(onsend).toHaveBeenCalledTimes(2);
  });

  it('offers the standard methods', () => {
    render(UrlBar, { method: 'PUT', url: '', sending: false, error: null, onsend: vi.fn() });
    const select = screen.getByLabelText('Method') as HTMLSelectElement;
    expect(select.value).toBe('PUT');
    expect([...select.options].map((option) => option.value)).toContain('DELETE');
  });

  it('is disabled while sending and shows errors', () => {
    render(UrlBar, {
      method: 'GET',
      url: 'x',
      sending: true,
      error: { key: 'error.url_empty', args: {} },
      onsend: vi.fn(),
    });
    expect(screen.getByRole('button', { name: 'Sending …' })).toBeDisabled();
    expect(screen.getByRole('alert')).toHaveTextContent('Please enter a URL.');
    expect(screen.getByLabelText('URL')).toHaveAttribute('aria-invalid', 'true');
  });
});
