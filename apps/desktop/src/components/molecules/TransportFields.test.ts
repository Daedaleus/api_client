import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { TransportDraft } from '../../lib/types';
import TransportFields from './TransportFields.svelte';

function setup(root = false) {
  const transport: TransportDraft = { redirects: 'inherit', timeoutSeconds: null };
  render(TransportFields, { transport, root });
  return transport;
}

describe('TransportFields', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('sets redirects and a time limit, empty inheriting', async () => {
    const transport = setup();
    const redirects = screen.getByLabelText('Redirects') as HTMLSelectElement;
    expect([...redirects.options].map((option) => option.text)).toEqual([
      'Inherit',
      'Follow',
      'Do not follow (show the 3xx)',
    ]);
    await userEvent.selectOptions(redirects, 'stop');
    const timeout = screen.getByLabelText('Time limit');
    expect(timeout).toHaveAttribute('placeholder', 'Inherit');
    await userEvent.type(timeout, '15');
    expect(transport).toEqual({ redirects: 'stop', timeoutSeconds: 15 });
    await userEvent.clear(timeout);
    expect(transport.timeoutSeconds).toBeNull();
  });

  it('shows the defaults on the collection', () => {
    setup(true);
    const redirects = screen.getByLabelText('Redirects') as HTMLSelectElement;
    expect([...redirects.options].map((option) => option.text)).toEqual([
      'Follow',
      'Do not follow (show the 3xx)',
    ]);
    expect(screen.getByLabelText('Time limit')).toHaveAttribute('placeholder', 'Default (30)');
  });
});
