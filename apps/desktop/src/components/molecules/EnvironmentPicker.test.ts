import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { shop } from '../../test/fake-api';
import EnvironmentPicker from './EnvironmentPicker.svelte';

function setup(chosen: string | null) {
  const props = { onchoose: vi.fn(), onedit: vi.fn(), onnew: vi.fn(), onrecipients: vi.fn() };
  render(EnvironmentPicker, { environments: shop().environments, chosen, ...props });
  return props;
}

describe('EnvironmentPicker', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('chooses an environment or none', async () => {
    const { onchoose } = setup(null);
    const select = screen.getByLabelText('Environment');
    expect(select).toHaveValue('');
    expect(screen.queryByRole('button', { name: 'Edit environment' })).not.toBeInTheDocument();

    await userEvent.selectOptions(select, 'Dev');
    expect(onchoose).toHaveBeenLastCalledWith('e1');
    await userEvent.selectOptions(select, 'No environment');
    expect(onchoose).toHaveBeenLastCalledWith(null);
  });

  it('edits the chosen environment and creates new ones', async () => {
    const { onedit, onnew, onrecipients } = setup('e1');
    expect(screen.getByLabelText('Environment')).toHaveValue('e1');
    await userEvent.click(screen.getByRole('button', { name: 'Edit environment' }));
    expect(onedit).toHaveBeenCalledWith('e1');
    await userEvent.click(screen.getByRole('button', { name: 'New environment' }));
    expect(onnew).toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Secret recipients' }));
    expect(onrecipients).toHaveBeenCalled();
  });
});
