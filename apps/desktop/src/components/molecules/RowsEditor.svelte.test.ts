import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import RowsEditor, { type Row, type RowLabels } from './RowsEditor.svelte';

const HEADERS: RowLabels = {
  enabled: 'editor.header_enabled',
  name: 'editor.header_name',
  value: 'editor.header_value',
  add: 'editor.add_header',
  remove: 'editor.remove_header',
  empty: 'editor.no_headers',
};

function setup(rows: Row[], labels = HEADERS, textMode = true) {
  const state = $state<{ rows: Row[] }>({ rows });
  render(RowsEditor, {
    get rows() {
      return state.rows;
    },
    set rows(next) {
      state.rows = next;
    },
    labels,
    textMode,
  });
  return state;
}

describe('RowsEditor as text', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('switches to text and back, reading pasted lines', async () => {
    const state = setup([
      { name: 'Accept', value: 'application/json', enabled: true },
      { name: 'X-Old', value: '1', enabled: false },
    ]);
    await userEvent.click(screen.getByRole('button', { name: 'Edit as text' }));
    const text = screen.getByRole('textbox', { name: 'Entries as text' });
    expect(text).toHaveValue('Accept: application/json\n// X-Old: 1');
    expect(screen.queryByRole('button', { name: 'Add header' })).toBeNull();
    expect(screen.getByText(/switches a line off/)).toBeInTheDocument();

    await fireEvent.input(text, {
      target: { value: 'Accept: text/html\ncookie: a=b\n// X-Debug: 1\n' },
    });
    expect(state.rows).toEqual([
      { name: 'Accept', value: 'text/html', enabled: true },
      { name: 'cookie', value: 'a=b', enabled: true },
      { name: 'X-Debug', value: '1', enabled: false },
    ]);

    await userEvent.click(screen.getByRole('button', { name: 'Edit as table' }));
    const names = screen.getAllByRole('textbox', { name: 'Header name' });
    expect(names.map((input) => (input as HTMLInputElement).value)).toEqual([
      'Accept',
      'cookie',
      'X-Debug',
    ]);
    expect(screen.getByRole('button', { name: 'Add header' })).toBeInTheDocument();
  });

  it('is not offered without text mode', () => {
    setup([], HEADERS, false);
    expect(screen.queryByRole('button', { name: 'Edit as text' })).toBeNull();
  });

  it('is never offered where rows may be secret', () => {
    setup([], {
      ...HEADERS,
      secret: { toggle: 'env.variable_secret', placeholder: 'env.secret_placeholder' },
    });
    expect(screen.queryByRole('button', { name: 'Edit as text' })).toBeNull();
  });
});
