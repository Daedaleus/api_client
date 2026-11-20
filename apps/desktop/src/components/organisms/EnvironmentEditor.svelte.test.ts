import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { EnvironmentDraft, UserText } from '../../lib/types';
import EnvironmentEditor from './EnvironmentEditor.svelte';

function setup(readOnly = false, error: UserText | null = null) {
  const draft: EnvironmentDraft = $state({
    id: 'e1',
    name: 'Dev',
    readOnly,
    variables: [
      { name: 'baseUrl', value: 'http://localhost', enabled: true, secret: false },
      { name: 'token', value: '', enabled: true, secret: true },
    ],
  });
  const props = { onsave: vi.fn(), oncancel: vi.fn(), ondelete: vi.fn() };
  render(EnvironmentEditor, { draft, error, ...props });
  return { draft, ...props };
}

describe('EnvironmentEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits name and variables and saves', async () => {
    const { draft, onsave } = setup();
    await userEvent.type(screen.getByLabelText('Name'), ' 2');
    await userEvent.click(screen.getByRole('button', { name: 'Add variable' }));
    await userEvent.type(screen.getAllByLabelText('Variable name')[2]!, 'apiKey');
    expect(draft.name).toBe('Dev 2');
    expect(draft.variables.map((variable) => variable.name)).toEqual([
      'baseUrl',
      'token',
      'apiKey',
    ]);
    expect(draft.variables[2]?.secret).toBe(false);

    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(onsave).toHaveBeenCalled();
  });

  it('cancels with the button or Escape and deletes', async () => {
    const { oncancel, ondelete } = setup();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await userEvent.keyboard('{Escape}');
    expect(oncancel).toHaveBeenCalledTimes(2);
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(ondelete).toHaveBeenCalled();
  });

  it('shows errors and blocks saving read-only environments', () => {
    setup(true, { key: 'error.write_failed', args: { path: 'dev.toml', reason: 'x' } });
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/newer format/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('masks secret values and marks variables as secret', async () => {
    const { draft } = setup();
    const values = screen.getAllByLabelText('Value');
    expect(values[0]).toHaveAttribute('type', 'text');
    expect(values[1]).toHaveAttribute('type', 'password');
    expect(values[1]).toHaveAttribute('placeholder', 'Encrypted – leave empty to keep');

    const toggles = screen.getAllByRole('checkbox', { name: 'Secret (stored encrypted)' });
    expect(toggles[1]).toBeChecked();
    await userEvent.click(toggles[0]!);
    expect(draft.variables[0]?.secret).toBe(true);
    expect(screen.getAllByLabelText('Value')[0]).toHaveAttribute('type', 'password');
  });
});
