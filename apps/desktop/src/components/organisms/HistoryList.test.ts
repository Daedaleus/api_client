import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { Workspace } from '../../lib/workspace/workspace.svelte';
import { FakeApi, draft } from '../../test/fake-api';
import Sidebar from './Sidebar.svelte';

const AT = new Date(2026, 10, 16, 9, 5).getTime();

function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  render(Sidebar, { workspace });
  return { api, workspace };
}

describe('HistoryList', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('switches the sidebar to the history, filters and opens entries', async () => {
    const { api, workspace } = setup();
    api.queryAnswers.push({
      type: 'history',
      entries: [
        {
          id: 'h1',
          atMs: AT,
          method: 'POST',
          url: 'https://api.example.com/login',
          collection: 'Shop',
          status: 201,
          elapsedMs: 42,
          error: null,
        },
        {
          id: 'h2',
          atMs: AT - 1000,
          method: 'GET',
          url: 'https://down.example.com/',
          collection: null,
          status: null,
          elapsedMs: null,
          error: { key: 'error.request', args: { detail: 'refused' } },
        },
      ],
    });
    await userEvent.click(screen.getByRole('tab', { name: 'History' }));
    expect(screen.getByRole('tab', { name: 'History' })).toHaveAttribute('aria-selected', 'true');
    const day = await screen.findByRole('region', { name: /2026/ });
    expect(within(day).getByText('201')).toBeInTheDocument();
    expect(within(day).getByText('Failed')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Open collection …' })).not.toBeInTheDocument();

    await userEvent.type(screen.getByRole('searchbox', { name: 'Filter history' }), 'down');
    expect(screen.queryByText('https://api.example.com/login')).not.toBeInTheDocument();
    await userEvent.clear(screen.getByRole('searchbox', { name: 'Filter history' }));
    await userEvent.type(screen.getByRole('searchbox', { name: 'Filter history' }), 'nothing');
    expect(screen.getByText('No entry matches the filter.')).toBeInTheDocument();
    await userEvent.clear(screen.getByRole('searchbox', { name: 'Filter history' }));

    api.queryAnswers.push({
      type: 'historyEntry',
      request: draft('', 'https://api.example.com/login'),
      outcome: {
        kind: 'failed',
        error: { key: 'error.request', args: { detail: 'x' } },
        logs: [],
        tests: [],
      },
    });
    await userEvent.click(
      screen.getByRole('button', { name: 'Open POST https://api.example.com/login' }),
    );
    expect(workspace.tabs.tabs).toHaveLength(2);

    await userEvent.click(screen.getByRole('tab', { name: 'Collections' }));
    expect(screen.getByRole('button', { name: 'Open collection …' })).toBeInTheDocument();
  });

  it('explains an empty history and shows errors', async () => {
    const { api } = setup();
    api.queryAnswers.push({ fail: { key: 'error.history', args: { detail: 'locked' } } });
    await userEvent.click(screen.getByRole('tab', { name: 'History' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('locked');
    expect(screen.getByText(/No requests sent yet/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Close message' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
