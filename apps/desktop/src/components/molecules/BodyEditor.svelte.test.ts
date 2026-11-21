import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { BodyDraft } from '../../lib/types';
import BodyEditor from './BodyEditor.svelte';

function setup(initial: BodyDraft, onpickfile?: () => Promise<string | null>) {
  const state = $state<{ body: BodyDraft }>({ body: initial });
  render(BodyEditor, {
    get body() {
      return state.body;
    },
    set body(next) {
      state.body = next;
    },
    onpickfile,
  });
  return state;
}

describe('BodyEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits form fields', async () => {
    const state = setup({ kind: 'none' });
    await userEvent.click(screen.getByRole('radio', { name: 'Form' }));
    expect(screen.getByText('No fields.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add field' }));
    await userEvent.type(screen.getByLabelText('Field name'), 'q');
    await userEvent.type(screen.getByLabelText('Value'), 'tea');
    expect(state.body).toEqual({
      kind: 'form',
      fields: [{ name: 'q', value: 'tea', enabled: true }],
    });
  });

  it('edits multipart text and file fields, picking files', async () => {
    const pick = vi.fn((): Promise<string | null> => Promise.resolve('C:\\files\\avatar.png'));
    const state = setup({ kind: 'multipart', fields: [] }, pick);
    expect(screen.getByText('No fields.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add field' }));
    await userEvent.type(screen.getByLabelText('Field name'), 'avatar');
    expect(screen.queryByRole('button', { name: 'Choose …' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('checkbox', { name: 'File' }));
    expect(screen.getByLabelText('Value')).toHaveAttribute('placeholder', 'Path of the file');
    await userEvent.click(screen.getByRole('button', { name: 'Choose …' }));
    expect(state.body).toEqual({
      kind: 'multipart',
      fields: [{ name: 'avatar', value: 'C:\\files\\avatar.png', enabled: true, file: true }],
    });

    pick.mockResolvedValueOnce(null);
    await userEvent.click(screen.getByRole('button', { name: 'Choose …' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Send' }));
    expect(state.body).toMatchObject({
      fields: [{ value: 'C:\\files\\avatar.png', enabled: false }],
    });
    await userEvent.click(screen.getByRole('button', { name: 'Remove field' }));
    expect(state.body).toEqual({ kind: 'multipart', fields: [] });
  });

  it('takes a file path for a binary body, typed or picked', async () => {
    const pick = vi.fn((): Promise<string | null> => Promise.resolve('/data/payload.bin'));
    const state = setup({ kind: 'binary', file: '' }, pick);
    expect(screen.getByText(/Relative paths start at the collection folder/)).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Path'), 'data.bin');
    expect(state.body).toEqual({ kind: 'binary', file: 'data.bin' });
    await userEvent.click(screen.getByRole('button', { name: 'Choose …' }));
    expect(state.body).toEqual({ kind: 'binary', file: '/data/payload.bin' });
    pick.mockResolvedValueOnce(null);
    await userEvent.click(screen.getByRole('button', { name: 'Choose …' }));
    expect(state.body).toEqual({ kind: 'binary', file: '/data/payload.bin' });
  });

  it('offers no picker without one', () => {
    setup({ kind: 'binary', file: '' });
    expect(screen.queryByRole('button', { name: 'Choose …' })).not.toBeInTheDocument();
  });
});
