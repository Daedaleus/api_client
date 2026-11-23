import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import DocsEditor from './DocsEditor.svelte';

function setup(docs: string) {
  const state = $state({ docs });
  render(DocsEditor, {
    get docs() {
      return state.docs;
    },
    set docs(next) {
      state.docs = next;
    },
  });
  return state;
}

describe('DocsEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('starts writing when empty and shows the result', async () => {
    const state = setup('');
    const text = screen.getByRole('textbox', { name: 'Documentation (Markdown)' });
    expect(screen.getByText(/images are not loaded/)).toBeInTheDocument();
    await userEvent.type(text, '# Login');
    expect(state.docs).toBe('# Login');
    await userEvent.click(screen.getByRole('button', { name: 'Preview' }));
    expect(screen.getByRole('heading', { name: 'Login' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Preview' })).toHaveAttribute('aria-pressed', 'true');

    state.docs = '  ';
    expect(await screen.findByText('No documentation yet.')).toBeInTheDocument();
  });

  it('starts reading existing docs; links do not navigate', async () => {
    setup('See [the API](https://example.com/api).');
    const link = screen.getByRole('link', { name: 'the API' });
    expect(link).toHaveAttribute('title', 'https://example.com/api');
    const allowed = await fireEvent.click(link);
    // `false`: the default action (navigating the window) was prevented.
    expect(allowed).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByRole('textbox', { name: 'Documentation (Markdown)' })).toHaveValue(
      'See [the API](https://example.com/api).',
    );
  });
});
