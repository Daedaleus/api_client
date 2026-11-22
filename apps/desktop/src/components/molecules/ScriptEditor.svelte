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
  import { EditorState, type Extension } from '@codemirror/state';
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
        // Ctrl+Enter sends the request (workspace shortcut), not a new line.
        ...defaultKeymap.filter((binding) => binding.key !== 'Mod-Enter'),
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
