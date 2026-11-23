<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { renderMarkdown } from '../../lib/markdown';

  interface Props {
    /** Markdown, edited in place. */
    docs: string;
  }

  let { docs = $bindable() }: Props = $props();

  /** Writing (else reading); empty docs start in writing. */
  let editing = $state(docs.trim() === '');
  let html = $derived(renderMarkdown(docs));
  let preview = $state<HTMLElement>();

  // Links show their address but never navigate the app window.
  $effect(() => {
    const element = preview;
    if (!element) return;
    const stay = (event: Event) => {
      if (event.target instanceof Element && event.target.closest('a')) event.preventDefault();
    };
    element.addEventListener('click', stay);
    return () => element.removeEventListener('click', stay);
  });
</script>

<div class="docs-editor">
  <div class="docs-editor__modes" role="group" aria-label={i18n.t('docs.mode')}>
    <button
      type="button"
      class="docs-editor__mode"
      class:docs-editor__mode--active={!editing}
      aria-pressed={!editing}
      onclick={() => (editing = false)}
    >
      {i18n.t('docs.preview')}
    </button>
    <button
      type="button"
      class="docs-editor__mode"
      class:docs-editor__mode--active={editing}
      aria-pressed={editing}
      onclick={() => (editing = true)}
    >
      {i18n.t('docs.edit')}
    </button>
  </div>
  {#if editing}
    <textarea
      class="docs-editor__text"
      rows="14"
      aria-label={i18n.t('docs.label')}
      placeholder={i18n.t('docs.placeholder')}
      bind:value={docs}></textarea>
    <p class="docs-editor__hint">{i18n.t('docs.hint')}</p>
  {:else if docs.trim() === ''}
    <p class="docs-editor__hint">{i18n.t('docs.empty')}</p>
  {:else}
    <div class="docs-editor__preview" bind:this={preview}>
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- sanitized by renderMarkdown (DOMPurify) -->
      {@html html}
    </div>
  {/if}
</div>

<style>
  .docs-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .docs-editor__modes {
    display: flex;
    gap: var(--space-xs);
  }

  .docs-editor__mode {
    padding: 2px var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-muted);
    cursor: pointer;
  }

  .docs-editor__mode--active {
    background: var(--color-raised);
    color: var(--color-text);
  }

  .docs-editor__text {
    width: 100%;
    padding: var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
    resize: vertical;
  }

  .docs-editor__hint {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .docs-editor__preview {
    line-height: 1.6;
    overflow-wrap: anywhere;
  }

  /* Rendered Markdown: element selectors are unavoidable for generated HTML. */
  .docs-editor__preview :global(code) {
    padding: 0 4px;
    border-radius: 4px;
    background: var(--color-raised);
    font-family: var(--font-mono);
  }

  .docs-editor__preview :global(pre) {
    padding: var(--space-s);
    border-radius: 6px;
    background: var(--color-raised);
    overflow-x: auto;
  }

  .docs-editor__preview :global(table) {
    border-collapse: collapse;
  }

  .docs-editor__preview :global(th),
  .docs-editor__preview :global(td) {
    padding: 2px var(--space-s);
    border: 1px solid var(--color-border);
  }

  .docs-editor__preview :global(a) {
    color: var(--color-accent);
    cursor: default;
  }
</style>
