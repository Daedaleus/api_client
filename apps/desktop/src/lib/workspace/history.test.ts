import { describe, expect, it, vi } from 'vitest';

import { FakeApi, draft } from '../../test/fake-api';
import type { HistoryItemView } from '../types';
import { groupByDay } from '../history';
import { Workspace } from './workspace.svelte';

function item(id: string, atMs: number, overrides: Partial<HistoryItemView> = {}): HistoryItemView {
  return {
    id,
    atMs,
    method: 'GET',
    url: `https://api.example.com/${id}`,
    collection: null,
    status: 200,
    elapsedMs: 12,
    error: null,
    ...overrides,
  };
}

/** Answers the confirmation the history asked for. */
function answer(workspace: Workspace, ok: boolean) {
  const dialog = workspace.dialogs.current;
  if (dialog?.kind !== 'confirm') throw new Error('no confirmation asked');
  dialog.answer(ok);
}

const MONDAY_NOON = new Date(2026, 10, 16, 12, 0).getTime();
const MONDAY_MORNING = new Date(2026, 10, 16, 8, 30).getTime();
const SUNDAY = new Date(2026, 10, 15, 23, 59).getTime();

describe('request history', () => {
  it('groups entries by local day and filters them', () => {
    const entries = [
      item('a', MONDAY_NOON, { method: 'POST', collection: 'Shop' }),
      item('b', MONDAY_MORNING, { status: 404 }),
      item('c', SUNDAY, { status: null, elapsedMs: null }),
    ];
    const days = groupByDay(entries, '');
    expect(days.map((day) => day.entries.map((entry) => entry.id))).toEqual([['a', 'b'], ['c']]);
    expect(days[0]!.day).toBe(new Date(2026, 10, 16).getTime());
    expect(groupByDay(entries, ' post ').map((day) => day.entries[0]!.id)).toEqual(['a']);
    expect(groupByDay(entries, '404')[0]!.entries[0]!.id).toBe('b');
    expect(groupByDay(entries, 'shop')[0]!.entries[0]!.id).toBe('a');
    expect(groupByDay(entries, 'nothing')).toEqual([]);
  });

  it('loads when shown, refreshes on changes, opens and clears', async () => {
    const api = new FakeApi();
    const workspace = new Workspace(api, null);
    await workspace.connect();
    const history = workspace.history;

    // Hidden: changes do not query.
    await history.changed(null);
    expect(api.queries).toEqual([]);

    api.queryAnswers.push({ type: 'history', entries: [item('a', MONDAY_NOON)] });
    await history.show(true);
    expect(history.entries.map((entry) => entry.id)).toEqual(['a']);

    api.queryAnswers.push({
      type: 'history',
      entries: [item('b', MONDAY_NOON + 1), item('a', MONDAY_NOON)],
    });
    api.handler!({
      type: 'historyChanged',
      error: { key: 'error.history', args: { detail: 'x' } },
    });
    await vi.waitFor(() => expect(history.entries).toHaveLength(2));
    expect(history.error?.key).toBe('error.history');

    api.queryAnswers.push({
      type: 'historyEntry',
      request: draft('', 'https://api.example.com/b'),
      outcome: {
        kind: 'failed',
        error: { key: 'error.request', args: { detail: 'down' } },
        logs: [],
        tests: [],
      },
    });
    await history.open('b');
    const tab = workspace.tabs.active!;
    expect(tab.saved).toBeNull();
    expect(tab.outcome?.kind).toBe('failed');
    expect(api.queries.at(-1)).toEqual({ type: 'historyEntry', id: 'b' });

    // Cancelled: nothing happens; confirmed: cleared.
    const cancel = history.clear();
    answer(workspace, false);
    await cancel;
    expect(api.commands).toEqual([]);
    api.answers.push({ type: 'historyCleared' });
    const confirm = history.clear();
    answer(workspace, true);
    await confirm;
    expect(api.commands).toEqual([{ type: 'clearHistory' }]);
    expect(history.entries).toEqual([]);

    await history.show(false);
    expect(history.visible).toBe(false);
  });

  it('reports failures', async () => {
    const api = new FakeApi();
    const workspace = new Workspace(api, null);
    const history = workspace.history;
    api.queryAnswers.push({ fail: { key: 'error.history', args: { detail: 'read' } } });
    await history.refresh();
    expect(history.error?.args['detail']).toBe('read');
    api.queryAnswers.push({ fail: { key: 'error.unknown_item', args: {} } });
    await history.open('gone');
    expect(history.error?.key).toBe('error.unknown_item');
    api.answers.push({ fail: { key: 'error.history', args: { detail: 'write' } } });
    const clearing = history.clear();
    answer(workspace, true);
    await clearing;
    expect(history.error?.args['detail']).toBe('write');
  });
});
