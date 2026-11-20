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
    variables: [{ name: 'baseUrl', value: 'http://localhost', enabled: true }],
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
    await userEvent.type(screen.getAllByLabelText('Variable name')[1]!, 'token');
    expect(draft.name).toBe('Dev 2');
    expect(draft.variables.map((variable) => variable.name)).toEqual(['baseUrl', 'token']);

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
});
