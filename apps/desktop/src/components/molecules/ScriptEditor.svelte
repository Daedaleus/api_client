<script lang="ts">
  import { onMount } from 'svelte';
  import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    completionKeymap,
  } from '@codemirror/autocomplete';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { javascript } from '@codemirror/lang-javascript';
  import { bracketMatching, indentOnInput } from '@codemirror/language';
  import { linter, lintGutter, lintKeymap } from '@codemirror/lint';
  import { EditorState, StateEffect, type Extension } from '@codemirror/state';
  import {
    EditorView,
    keymap,
    lineNumbers,
    placeholder as placeholderText,
  } from '@codemirror/view';

  import { codeTheme } from '../../lib/codemirror-theme';
  import { complete, toDiagnostics, type ScriptAssist } from '../../lib/script-assist';

  interface Props {
    value: string;
    /** Accessible name of the editor. */
    label: string;
    placeholder?: string;
    /** Checker and completion; without it a plain code editor. */
    assist?: ScriptAssist | undefined;
  }

  let { value = $bindable(), label, placeholder = '', assist }: Props = $props();

  let host = $state<HTMLDivElement>();
  let view: EditorView | undefined;

  /** Wait this long after typing before checking (ms). */
  const CHECK_DELAY = 400;
  /** Milliseconds before checking again while the checker starts. */
  const RECHECK_DELAY = 500;
  /** Asks the linter to check the unchanged text again. */
  const recheck = StateEffect.define<null>();

  /** The script editor is at least a few lines high. */
  const size = EditorView.theme({ '.cm-content': { minHeight: '7.5em' } });

  function extensions(): Extension[] {
    const base: Extension[] = [
      lineNumbers(),
      history(),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      javascript({ typescript: true }),
      placeholderText(placeholder),
      // Tab is left to move the focus (keyboard users must get out again).
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...historyKeymap,
        ...completionKeymap,
        ...lintKeymap,
      ]),
      EditorView.contentAttributes.of({ 'aria-label': label }),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) value = update.state.doc.toString();
      }),
      codeTheme,
      size,
    ];
    if (!assist) return base;
    const checker = assist;
    return [
      ...base,
      lintGutter(),
      linter(
        async (current) => {
          const checked = await checker.check(current.state.doc.toString());
          // Syntax errors only for now: the full check follows once the
          // checker has started.
          if (checked.partial) {
            setTimeout(() => current.dispatch({ effects: recheck.of(null) }), RECHECK_DELAY);
          }
          return toDiagnostics(checked.items, current.state.doc.length);
        },
        {
          delay: CHECK_DELAY,
          needsRefresh: (update) =>
            update.transactions.some((transaction) =>
              transaction.effects.some((effect) => effect.is(recheck)),
            ),
        },
      ),
      autocompletion({ override: [(context) => complete(checker, context)] }),
    ];
  }

  onMount(() => {
    view = new EditorView({
      parent: host!,
      state: EditorState.create({ doc: value, extensions: extensions() }),
    });
    return () => view?.destroy();
  });

  // Changes from outside (another tab's draft) replace the text.
  $effect(() => {
    const text = value;
    if (view && text !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    }
  });
</script>

<div class="script-editor" bind:this={host}></div>

<style>
  .script-editor {
    min-width: 0;
  }
</style>
