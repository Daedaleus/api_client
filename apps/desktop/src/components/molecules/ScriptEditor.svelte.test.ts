import { currentCompletions, startCompletion } from '@codemirror/autocomplete';
import { diagnosticCount, forceLinting } from '@codemirror/lint';
import { EditorView } from '@codemirror/view';
import { render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import type { ScriptAssist } from '../../lib/script-assist';
import ScriptEditor from './ScriptEditor.svelte';

function setup(value: string, assist?: ScriptAssist) {
  const state = $state({ value });
  const result = render(ScriptEditor, {
    get value() {
      return state.value;
    },
    set value(next) {
      state.value = next;
    },
    label: 'Before the request',
    placeholder: 'log(1)',
    assist,
  });
  const content = screen.getByRole('textbox', { name: 'Before the request' });
  const view = EditorView.findFromDOM(content)!;
  return { state, view, result };
}

describe('ScriptEditor', () => {
  it('edits the bound text both ways', async () => {
    const { state, view } = setup("log('a')");
    expect(view.state.doc.toString()).toBe("log('a')");
    view.dispatch({ changes: { from: view.state.doc.length, insert: ';' } });
    expect(state.value).toBe("log('a');");
    state.value = 'env.get("x")';
    await waitFor(() => expect(view.state.doc.toString()).toBe('env.get("x")'));
  });

  it('marks what the checker finds and offers its completions', async () => {
    const assist: ScriptAssist = {
      check: vi.fn(async () => [
        { from: 4, to: 9, severity: 'error' as const, message: "Property 'methd' does not exist" },
      ]),
      complete: vi.fn(async () => [{ label: 'setHeader', kind: 'function' as const }]),
    };
    const { view } = setup('req.methd', assist);
    forceLinting(view);
    await waitFor(() => expect(diagnosticCount(view.state)).toBe(1));
    expect(assist.check).toHaveBeenCalledWith('req.methd');

    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: 'req.' },
      selection: { anchor: 4 },
    });
    startCompletion(view);
    await waitFor(() =>
      expect(currentCompletions(view.state).map((option) => option.label)).toEqual(['setHeader']),
    );
  });

  it('is a plain editor without a checker', () => {
    const { view } = setup('');
    forceLinting(view);
    expect(diagnosticCount(view.state)).toBe(0);
    expect(screen.getByText('log(1)')).toBeInTheDocument();
  });
});
