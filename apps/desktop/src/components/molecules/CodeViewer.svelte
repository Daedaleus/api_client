<script lang="ts">
  import { onMount } from 'svelte';
  import { defaultKeymap } from '@codemirror/commands';
  import { json } from '@codemirror/lang-json';
  import { codeFolding, foldGutter, foldKeymap } from '@codemirror/language';
  import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search';
  import { EditorState, type Extension } from '@codemirror/state';
  import { EditorView, keymap, lineNumbers } from '@codemirror/view';

  import { codeTheme } from '../../lib/codemirror-theme';

  interface Props {
    text: string;
    /** Highlight and fold as JSON. */
    isJson?: boolean;
    /** Accessible name. */
    label: string;
  }

  let { text, isJson = false, label }: Props = $props();

  let host = $state<HTMLDivElement>();
  let view: EditorView | undefined;

  /** Fills its panel; only the visible lines are drawn, so large bodies stay quick. */
  const layout = EditorView.theme({
    '&': { height: '100%' },
    '.cm-scroller': { overflow: 'auto' },
  });

  function viewState(content: string, asJson: boolean): EditorState {
    const extensions: Extension[] = [
      EditorState.readOnly.of(true),
      lineNumbers(),
      codeFolding(),
      foldGutter(),
      search({ top: true }),
      highlightSelectionMatches(),
      keymap.of([...searchKeymap, ...foldKeymap, ...defaultKeymap]),
      EditorView.contentAttributes.of({ 'aria-label': label, 'aria-readonly': 'true' }),
      codeTheme,
      layout,
    ];
    if (asJson) extensions.push(json());
    return EditorState.create({ doc: content, extensions });
  }

  onMount(() => {
    view = new EditorView({ parent: host!, state: viewState(text, isJson) });
    return () => view?.destroy();
  });

  // A new response replaces the whole view (text and language).
  $effect(() => {
    const content = text;
    const asJson = isJson;
    if (view && content !== view.state.doc.toString()) view.setState(viewState(content, asJson));
  });
</script>

<div class="code-viewer" bind:this={host} data-length={text.length}></div>

<style>
  .code-viewer {
    height: 100%;
    min-height: 0;
  }
</style>
