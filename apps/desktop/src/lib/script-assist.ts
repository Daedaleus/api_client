// The script editor's checker and completion (ADR S2): what the backend
// answers, turned into CodeMirror diagnostics and completions. Positions
// are JavaScript string offsets on both sides.

import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import type { Diagnostic } from '@codemirror/lint';

import type { Api } from './api';
import { toUserText } from './errors';
import { i18n } from './i18n/i18n.svelte';
import type { CompletionView, DiagnosticView } from './types';

/** Checks and completes scripts. */
export interface ScriptAssist {
  check(source: string): Promise<DiagnosticView[]>;
  complete(source: string, offset: number): Promise<CompletionView[]>;
}

/** The backend's checker. A checker that cannot answer is reported as a note at the start. */
export function apiAssist(api: Api): ScriptAssist {
  return {
    async check(source) {
      try {
        const result = await api.assist({ type: 'check', source });
        return result.type === 'diagnostics' ? result.items : [];
      } catch (error) {
        return [{ from: 0, to: 0, severity: 'info', message: i18n.text(toUserText(error)) }];
      }
    },
    async complete(source, offset) {
      try {
        const result = await api.assist({ type: 'complete', source, offset });
        return result.type === 'completions' ? result.items : [];
      } catch {
        return [];
      }
    },
  };
}

/** The checker's findings as CodeMirror diagnostics, inside a document of `length`. */
export function toDiagnostics(items: DiagnosticView[], length: number): Diagnostic[] {
  return items.map((item) => {
    const from = Math.min(item.from, length);
    return {
      from,
      to: Math.max(from, Math.min(item.to, length)),
      severity: item.severity,
      message: item.message,
    };
  });
}

/** Characters of names that completions replace. */
const WORD = /[\w$]*/;

/**
 * Suggestions at the cursor: after a `.`, while typing a name, or when asked
 * (Ctrl+Space). Nothing in the middle of nowhere.
 */
export async function complete(
  assist: ScriptAssist,
  context: CompletionContext,
): Promise<CompletionResult | null> {
  const word = context.matchBefore(WORD);
  if (!word) return null;
  const afterDot = context.state.sliceDoc(word.from - 1, word.from) === '.';
  if (word.from === word.to && !afterDot && !context.explicit) return null;
  const items = await assist.complete(context.state.doc.toString(), context.pos);
  if (items.length === 0) return null;
  return {
    from: word.from,
    options: items.map((item) => ({ label: item.label, type: item.kind })),
    validFor: /^[\w$]*$/,
  };
}
