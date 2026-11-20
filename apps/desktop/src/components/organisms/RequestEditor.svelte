<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import UrlBar from '../molecules/UrlBar.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { RequestTab } from '../../lib/workspace/request-tab.svelte';

  interface Props {
    tab: RequestTab;
    onsend: () => void;
    onsave: () => void;
  }

  let { tab, onsend, onsave }: Props = $props();
</script>

<div class="request-editor">
  {#if tab.saved}
    <div class="request-editor__header">
      <label class="request-editor__name">
        <span class="request-editor__label">{i18n.t('editor.name')}</span>
        <input class="request-editor__input" type="text" bind:value={tab.draft.name} />
      </label>
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

  .request-editor__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
