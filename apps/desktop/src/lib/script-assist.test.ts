import { CompletionContext } from '@codemirror/autocomplete';
import { EditorState } from '@codemirror/state';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FakeApi } from '../test/fake-api';
import { i18n } from './i18n/i18n.svelte';
import { apiAssist, complete, toDiagnostics, type ScriptAssist } from './script-assist';

/** A completion context with the cursor at `|`. */
function context(text: string, explicit = false): CompletionContext {
  const pos = text.indexOf('|');
  const state = EditorState.create({ doc: text.replace('|', '') });
  return new CompletionContext(state, pos, explicit);
}

function fakeAssist(labels: string[]): ScriptAssist & { complete: ReturnType<typeof vi.fn> } {
  return {
    check: vi.fn(async () => ({ items: [], partial: false })),
    complete: vi.fn(async () => labels.map((label) => ({ label, kind: 'function' as const }))),
  };
}

describe('script assist', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('asks the backend and reports a checker that cannot answer', async () => {
    const api = new FakeApi();
    const assist = apiAssist(api);
    api.assistAnswers.push({
      type: 'diagnostics',
      items: [{ from: 0, to: 3, severity: 'error', message: 'bad' }],
      partial: true,
    });
    expect(await assist.check('req')).toEqual({
      items: [{ from: 0, to: 3, severity: 'error', message: 'bad' }],
      partial: true,
    });
    api.assistAnswers.push({ fail: { key: 'error.script_assist', args: { detail: 'down' } } });
    expect(await assist.check('req')).toEqual({
      items: [{ from: 0, to: 0, severity: 'info', message: 'Script check not possible: down' }],
      partial: false,
    });
    api.assistAnswers.push({ type: 'completions', items: [] });
    expect(await assist.check('x')).toEqual({ items: [], partial: false });

    api.assistAnswers.push({
      type: 'completions',
      items: [{ label: 'setHeader', kind: 'function' }],
    });
    expect(await assist.complete('req.', 4)).toEqual([{ label: 'setHeader', kind: 'function' }]);
    api.assistAnswers.push({ fail: new Error('down') });
    expect(await assist.complete('req.', 4)).toEqual([]);
    api.assistAnswers.push({ type: 'diagnostics', items: [], partial: false });
    expect(await assist.complete('x', 1)).toEqual([]);
    expect(api.assisted.at(-1)).toEqual({ type: 'complete', source: 'x', offset: 1 });
  });

  it('keeps diagnostics inside the document', () => {
    expect(
      toDiagnostics(
        [
          { from: 2, to: 50, severity: 'warning', message: 'w' },
          { from: 90, to: 99, severity: 'info', message: 'i' },
        ],
        10,
      ),
    ).toEqual([
      { from: 2, to: 10, severity: 'warning', message: 'w' },
      { from: 10, to: 10, severity: 'info', message: 'i' },
    ]);
  });

  it('completes after a dot, while typing a name and when asked', async () => {
    const assist = fakeAssist(['setHeader', 'method']);
    const afterDot = await complete(assist, context('req.|'));
    expect(afterDot).toMatchObject({
      from: 4,
      options: [
        { label: 'setHeader', type: 'function' },
        { label: 'method', type: 'function' },
      ],
    });
    expect(assist.complete).toHaveBeenLastCalledWith('req.', 4);
    expect((await complete(assist, context('req.se|')))?.from).toBe(4);
    expect(await complete(assist, context('x = |'))).toBeNull();
    expect((await complete(assist, context('x = |', true)))?.from).toBe(4);
    expect(await complete(fakeAssist([]), context('req.|'))).toBeNull();
  });
});
