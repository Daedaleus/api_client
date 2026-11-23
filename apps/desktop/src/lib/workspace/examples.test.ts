import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import ExamplesPanel from '../../components/molecules/ExamplesPanel.svelte';
import { FakeApi, OK, draft } from '../../test/fake-api';
import { i18n } from '../i18n/i18n.svelte';
import type { ExampleView, ResponseView } from '../types';
import { Dialogs } from './dialogs.svelte';
import { ExamplesState } from './examples.svelte';
import { RequestTab } from './request-tab.svelte';

const RESPONSE: ResponseView = {
  status: 200,
  reason: 'OK',
  statusClass: 'success',
  elapsedMs: 0,
  sizeBytes: 4,
  headers: [],
  body: { text: 'nope', pretty: false, truncated: false, binary: false },
  logs: [],
  scriptError: null,
  tests: [],
};
const CREATED: ExampleView = {
  name: '201 Created',
  response: {
    ...RESPONSE,
    status: 201,
    reason: 'Created',
    headers: [{ name: 'Content-Type', value: 'application/json' }],
    body: { text: '{\n  "id": 1\n}', pretty: true, truncated: false, binary: false },
  },
};
const MISSING: ExampleView = {
  name: 'Missing',
  response: { ...RESPONSE, status: 404, reason: 'Not Found', statusClass: 'clientError' },
};

function setup() {
  const api = new FakeApi();
  const dialogs = new Dialogs();
  const state = new ExamplesState(api, dialogs);
  const tab = new RequestTab(draft('r1', 'Login'), { collectionId: 'c1', requestId: 'r1' });
  return { api, dialogs, state, tab };
}

/** Answers the open dialog (a name, or OK/cancel). */
function answer(dialogs: Dialogs, value: string | boolean | null) {
  const current = dialogs.current as { answer(value: unknown): void } | null;
  current?.answer(value);
}

describe('response examples', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('load once for saved requests only', async () => {
    const { api, state, tab } = setup();
    api.queryAnswers.push({ type: 'examples', examples: [CREATED] });
    await state.load(tab);
    await state.load(tab);
    expect(api.queries).toEqual([{ type: 'examples', collectionId: 'c1', requestId: 'r1' }]);
    expect(tab.examples).toEqual([CREATED]);

    const scratch = RequestTab.scratch();
    await state.load(scratch);
    expect(scratch.examples).toBeNull();

    const failing = new RequestTab(draft('r2', 'X'), { collectionId: 'c1', requestId: 'r2' });
    api.queryAnswers.push({ fail: { key: 'error.unknown_item', args: {} } });
    await state.load(failing);
    expect(failing.error?.key).toBe('error.unknown_item');
    api.queryAnswers.push({ type: 'history', entries: [] });
    await state.load(failing);
    expect(failing.error?.args.detail).toBe('unexpected answer history');
  });

  it('keep the shown response under a name the user gives', async () => {
    const { api, dialogs, state, tab } = setup();
    expect(state.canSave(tab)).toBe(false);
    tab.outcome = OK;
    tab.outcomeRun = 4;
    expect(state.canSave(tab)).toBe(true);
    expect(state.canSave(RequestTab.scratch())).toBe(false);

    const cancelled = state.saveFrom(tab);
    expect(dialogs.current).toMatchObject({ kind: 'name', value: '200 OK' });
    answer(dialogs, null);
    await cancelled;
    expect(api.commands).toEqual([]);

    api.answers.push({ type: 'examplesChanged', examples: [CREATED] });
    const saving = state.saveFrom(tab);
    answer(dialogs, 'Created');
    await saving;
    expect(api.commands.at(-1)).toEqual({
      type: 'saveExample',
      collectionId: 'c1',
      requestId: 'r1',
      runId: 4,
      name: 'Created',
    });
    expect(tab.examples).toEqual([CREATED]);

    api.answers.push({ fail: { key: 'error.example_not_text', args: {} } });
    const binary = state.saveFrom(tab);
    answer(dialogs, 'Image');
    await binary;
    expect(tab.error?.key).toBe('error.example_not_text');

    tab.outcomeRun = null;
    await state.saveFrom(tab);
    expect(dialogs.current).toBeNull();
  });

  it('remove an example after asking', async () => {
    const { api, dialogs, state, tab } = setup();
    tab.examples = [CREATED, MISSING];
    const kept = state.remove(tab, 1);
    expect(dialogs.current).toMatchObject({
      kind: 'confirm',
      title: { key: 'examples.delete_title', args: { name: 'Missing' } },
    });
    answer(dialogs, false);
    await kept;
    expect(api.commands).toEqual([]);

    api.answers.push({ type: 'examplesChanged', examples: [CREATED] });
    const removing = state.remove(tab, 1);
    answer(dialogs, true);
    await removing;
    expect(api.commands.at(-1)).toEqual({
      type: 'deleteExample',
      collectionId: 'c1',
      requestId: 'r1',
      index: 1,
    });
    expect(tab.examples).toEqual([CREATED]);
    await state.remove(tab, 5);
    expect(dialogs.current).toBeNull();
  });

  it('are listed and shown in the panel', async () => {
    const ondelete = vi.fn();
    const { rerender } = render(ExamplesPanel, { examples: null, ondelete });
    expect(screen.getByText('Loading examples …')).toBeInTheDocument();
    await rerender({ examples: [], ondelete });
    expect(screen.getByText(/No examples yet/)).toBeInTheDocument();

    await rerender({ examples: [CREATED, MISSING], ondelete });
    expect(screen.getByText('Content-Type: application/json')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '201 201 Created' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await userEvent.click(screen.getByRole('button', { name: '404 Missing' }));
    expect(screen.getByText('404 Not Found')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Delete example “Missing”' }));
    expect(ondelete).toHaveBeenCalledWith(1);
  });
});
