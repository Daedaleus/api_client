<script lang="ts">
  import type { Snippet } from 'svelte';
  import Button from '../atoms/Button.svelte';
  import BodyEditor from '../molecules/BodyEditor.svelte';
  import RowsEditor, { type RowLabels } from '../molecules/RowsEditor.svelte';
  import UrlBar from '../molecules/UrlBar.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { hasPlaceholders, preview } from '../../lib/variables';
  import type { RequestTab } from '../../lib/workspace/request-tab.svelte';

  interface Props {
    tab: RequestTab;
    onsend: () => void;
    onsave: () => void;
    /** Values of the chosen environment, `null` without one. */
    variables?: ReadonlyMap<string, string> | null;
    /** The environment picker, shown next to the name of a saved request. */
    environment?: Snippet;
  }

  let { tab, onsend, onsave, variables = null, environment }: Props = $props();

  // What the URL looks like with its `{{variables}}` filled in.
  let urlPreview = $derived(
    hasPlaceholders(tab.draft.url) && variables ? preview(tab.draft.url, variables) : null,
  );

  const HEADER_LABELS: RowLabels = {
    enabled: 'editor.header_enabled',
    name: 'editor.header_name',
    value: 'editor.header_value',
    add: 'editor.add_header',
    remove: 'editor.remove_header',
    empty: 'editor.no_headers',
  };

  type Section = 'headers' | 'body';
  const SECTIONS: Section[] = ['headers', 'body'];
  let section = $state<Section>('headers');
  let activeHeaders = $derived(
    tab.draft.headers.filter((header) => header.enabled && header.name.trim() !== '').length,
  );
</script>

<div class="request-editor">
  {#if tab.saved}
    <div class="request-editor__header">
      <label class="request-editor__name">
        <span class="request-editor__label">{i18n.t('editor.name')}</span>
        <input class="request-editor__input" type="text" bind:value={tab.draft.name} />
      </label>
      {@render environment?.()}
      <Button
        variant={tab.dirty ? 'primary' : 'secondary'}
        disabled={tab.saving || tab.draft.readOnly}
        onclick={onsave}
      >
        {tab.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
      </Button>
    </div>
    {#if tab.draft.readOnly}
      <p class="request-editor__note">{i18n.t('editor.read_only')}</p>
    {/if}
  {:else}
    <p class="request-editor__note">{i18n.t('editor.scratch')}</p>
  {/if}
  <UrlBar
    bind:method={tab.draft.method}
    bind:url={tab.draft.url}
    sending={tab.sending}
    error={tab.error}
    {onsend}
  />
  {#if urlPreview}
    <p class="request-editor__preview">
      {i18n.t('editor.preview', { url: urlPreview.text })}
    </p>
    {#if urlPreview.missing.length > 0}
      <p class="request-editor__warning">
        {i18n.t('editor.variables_missing', { names: urlPreview.missing.join(', ') })}
      </p>
    {/if}
  {:else if hasPlaceholders(tab.draft.url)}
    <p class="request-editor__warning">{i18n.t('editor.variables_no_environment')}</p>
  {/if}
  <div class="request-editor__sections" role="tablist" aria-label={i18n.t('editor.sections')}>
    {#each SECTIONS as name (name)}
      <button
        type="button"
        role="tab"
        class="request-editor__section"
        class:request-editor__section--active={section === name}
        aria-selected={section === name}
        onclick={() => (section = name)}
      >
        {i18n.t(`editor.${name}`)}
        {#if name === 'headers' && activeHeaders > 0}
          <span class="request-editor__count">{activeHeaders}</span>
        {:else if name === 'body' && tab.draft.body.kind !== 'none'}
          <span class="request-editor__count">{tab.draft.body.kind.toUpperCase()}</span>
        {/if}
      </button>
    {/each}
  </div>
  <div class="request-editor__panel" role="tabpanel">
    {#if section === 'headers'}
      <RowsEditor bind:rows={tab.draft.headers} labels={HEADER_LABELS} />
    {:else}
      <BodyEditor bind:body={tab.draft.body} />
    {/if}
  </div>
</div>

<style>
  .request-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .request-editor__header {
    display: flex;
    gap: var(--space-s);
    align-items: center;
  }

  .request-editor__name {
    flex: 1;
    min-width: 0;
  }

  .request-editor__label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .request-editor__input {
    width: 100%;
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .request-editor__input:hover,
  .request-editor__input:focus {
    border-color: var(--color-border);
    background: var(--color-surface);
  }

  .request-editor__sections {
    display: flex;
    gap: var(--space-xs);
    border-bottom: 1px solid var(--color-border);
  }

  .request-editor__section {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-s) var(--space-s);
    border: 0;
    background: transparent;
    color: var(--color-muted);
    font-weight: 500;
    cursor: pointer;
  }

  .request-editor__section--active {
    color: var(--color-text);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .request-editor__count {
    padding: 0 6px;
    border-radius: 9px;
    background: var(--color-raised);
    font-size: var(--font-size-s);
  }

  .request-editor__panel {
    max-height: 40vh;
    overflow-y: auto;
  }

  .request-editor__preview {
    margin: 0;
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    overflow-wrap: anywhere;
  }

  .request-editor__warning {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }

  .request-editor__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
