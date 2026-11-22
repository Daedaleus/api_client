// The look of every CodeMirror view (script editor, response body): colours
// and fonts from the design tokens, so light and dark theme both work.

import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

const frame = EditorView.theme({
  '&': {
    border: '1px solid var(--color-border)',
    borderRadius: '6px',
    backgroundColor: 'var(--color-code-bg)',
    color: 'var(--color-text)',
    fontSize: 'var(--font-size-s)',
  },
  '&.cm-focused': { outline: '2px solid var(--color-accent)', outlineOffset: '-1px' },
  '.cm-content': { fontFamily: 'var(--font-mono)', caretColor: 'var(--color-text)' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.5' },
  '.cm-gutters': {
    backgroundColor: 'var(--color-raised)',
    color: 'var(--color-muted)',
    border: 'none',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'var(--color-code-selection)',
  },
  '.cm-searchMatch': { backgroundColor: 'var(--color-code-selection)' },
  '.cm-panels': { backgroundColor: 'var(--color-raised)', color: 'var(--color-text)' },
  '.cm-panels input, .cm-panels button': { color: 'inherit' },
});

const colours = HighlightStyle.define([
  { tag: [tags.string, tags.special(tags.string)], color: 'var(--color-syntax-string)' },
  { tag: [tags.number, tags.bool, tags.null], color: 'var(--color-syntax-number)' },
  {
    tag: [tags.keyword, tags.operatorKeyword, tags.controlKeyword, tags.definitionKeyword],
    color: 'var(--color-syntax-keyword)',
  },
  { tag: [tags.propertyName, tags.typeName], color: 'var(--color-syntax-property)' },
  {
    tag: [tags.comment, tags.lineComment, tags.blockComment],
    color: 'var(--color-syntax-comment)',
    fontStyle: 'italic',
  },
  {
    tag: [tags.punctuation, tags.bracket, tags.separator],
    color: 'var(--color-syntax-punctuation)',
  },
]);

/** Frame, fonts and syntax colours of the app's code views. */
export const codeTheme: Extension = [frame, syntaxHighlighting(colours)];
