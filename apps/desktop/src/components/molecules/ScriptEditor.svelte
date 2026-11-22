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
  import {
    bracketMatching,
    defaultHighlightStyle,
    indentOnInput,
    syntaxHighlighting,
  } from '@codemirror/language';
  import { linter, lintGutter, lintKeymap } from '@codemirror/lint';
  import { EditorState, type Extension } from '@codemirror/state';
  import {
    EditorView,
    keymap,
    lineNumbers,
    placeholder as placeholderText,
  } from '@codemirror/view';

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

  /** Colours and fonts from the design tokens (light and dark theme). */
  const theme = EditorView.theme({
    '&': {
      border: '1px solid var(--color-border)',
      borderRadius: '6px',
      backgroundColor: 'var(--color-code-bg)',
      color: 'var(--color-text)',
      fontSize: 'var(--font-size-s)',
    },
    '&.cm-focused': { outline: '2px solid var(--color-accent)', outlineOffset: '-1px' },
    '.cm-content': {
      fontFamily: 'var(--font-mono)',
      minHeight: '7.5em',
      caretColor: 'var(--color-text)',
    },
    '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.5' },
    '.cm-gutters': {
      backgroundColor: 'var(--color-raised)',
      color: 'var(--color-muted)',
      border: 'none',
    },
  });

  function extensions(): Extension[] {
    const base: Extension[] = [
      lineNumbers(),
      history(),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
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
      theme,
    ];
    if (!assist) return base;
    const checker = assist;
    return [
      ...base,
      lintGutter(),
      linter(
        async (current) =>
          toDiagnostics(
            await checker.check(current.state.doc.toString()),
            current.state.doc.length,
          ),
        { delay: CHECK_DELAY },
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
