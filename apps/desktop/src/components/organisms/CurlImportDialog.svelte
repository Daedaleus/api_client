<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { CurlState } from '../../lib/workspace/curl.svelte';

  let { curl }: { curl: CurlState } = $props();

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void curl.import();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      curl.closeImport();
    }
  }
</script>

{#if curl.importing}
  {@const importing = curl.importing}
  <div class="curl-import">
    <div
      class="curl-import__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="curl-import-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <form class="curl-import__form" onsubmit={submit}>
        <h2 id="curl-import-title" class="curl-import__title">{i18n.t('curl.import_title')}</h2>
        <p class="curl-import__note">{i18n.t('curl.import_hint')}</p>
        <label class="curl-import__field">
          <span class="curl-import__label">{i18n.t('curl.command')}</span>
          <!-- svelte-ignore a11y_autofocus -->
          <textarea
            class="curl-import__text"
            spellcheck="false"
            autofocus
            bind:value={importing.text}></textarea>
        </label>
        {#if importing.error}
          <p class="curl-import__error" role="alert">{i18n.text(importing.error)}</p>
        {/if}
        <div class="curl-import__actions">
          <Button onclick={() => curl.closeImport()}>{i18n.t('dialog.cancel')}</Button>
          <Button type="submit" variant="primary" disabled={importing.text.trim() === ''}>
            {i18n.t('curl.open')}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .curl-import {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .curl-import__dialog {
    width: min(720px, calc(100% - 2 * var(--space-l)));
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .curl-import__form {
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .curl-import__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .curl-import__note {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .curl-import__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .curl-import__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .curl-import__text {
    min-height: 180px;
    padding: var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-mono);
    resize: vertical;
  }

  .curl-import__error {
    margin: 0;
    color: var(--color-error);
  }

  .curl-import__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
