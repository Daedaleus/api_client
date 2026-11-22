import { foldable } from '@codemirror/language';
import { openSearchPanel, searchPanelOpen } from '@codemirror/search';
import { EditorView } from '@codemirror/view';
import { render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import CodeViewer from './CodeViewer.svelte';

function viewOf(name: string): EditorView {
  return EditorView.findFromDOM(screen.getByRole('textbox', { name }))!;
}

describe('CodeViewer', () => {
  it('shows JSON read-only, foldable and searchable', () => {
    render(CodeViewer, { text: '{\n  "items": [\n    1\n  ]\n}', isJson: true, label: 'Body' });
    const view = viewOf('Body');
    expect(view.state.readOnly).toBe(true);
    expect(screen.getByRole('textbox', { name: 'Body' })).toHaveAttribute('aria-readonly', 'true');
    const first = view.state.doc.line(1);
    expect(foldable(view.state, first.from, first.to)).not.toBeNull();
    openSearchPanel(view);
    expect(searchPanelOpen(view.state)).toBe(true);
  });

  it('replaces its text when the response changes', async () => {
    const result = render(CodeViewer, { text: 'first', label: 'Body' });
    expect(viewOf('Body').state.doc.toString()).toBe('first');
    expect(result.container.querySelector('.code-viewer')).toHaveAttribute('data-length', '5');
    await result.rerender({ text: '{"second": true}', isJson: true, label: 'Body' });
    await waitFor(() => expect(viewOf('Body').state.doc.toString()).toBe('{"second": true}'));
    // Plain text is not folded.
    const plain = render(CodeViewer, { text: 'a\nb', label: 'Plain' });
    const view = viewOf('Plain');
    expect(foldable(view.state, 0, 1)).toBeNull();
    plain.unmount();
  });
});
