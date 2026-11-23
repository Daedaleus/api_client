<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import { fitsCodeView, formatBytes } from '../../lib/format';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { JsonPathError, queryJson } from '../../lib/jsonpath';
  import type { BodyView, UserText } from '../../lib/types';
  import CodeViewer from './CodeViewer.svelte';

  interface Props {
    body: BodyView;
    sizeBytes: number;
    /** Saves the whole body (as received) to a file; missing if it cannot be. */
    onsave?: (() => void) | undefined;
  }

  let { body, sizeBytes, onsave }: Props = $props();

  let filter = $state('');
  let copied = $state(false);

  /** The filter result as text, or why there is none. */
  let filtered = $derived.by((): { text: string } | { problem: UserText } | null => {
    const path = filter.trim();
    if (!body.pretty || path === '' || path === '$') return null;
    let document: unknown;
    try {
      document = JSON.parse(body.text);
    } catch {
      return { problem: { key: 'response.filter_unparsable', args: {} } };
    }
    try {
      const found = queryJson(document, path);
      if (found.length === 0) return { problem: { key: 'response.filter_none', args: {} } };
      return { text: JSON.stringify(found.length === 1 ? found[0] : found, null, 2) };
    } catch (error) {
      if (error instanceof JsonPathError) return { problem: { key: error.key, args: error.args } };
      throw error;
    }
  });

  /** What the code view shows and the copy button copies. */
  let shown = $derived(filtered !== null && 'text' in filtered ? filtered.text : body.text);

  async function copy() {
    try {
      await navigator.clipboard.writeText(shown);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // No clipboard access: the text is still selectable.
    }
  }
</script>

<div class="response-body">
  <div class="response-body__tools">
    {#if body.pretty}
      <input
        class="response-body__filter"
        type="search"
        spellcheck="false"
        aria-label={i18n.t('response.filter')}
        placeholder={i18n.t('response.filter_placeholder')}
        bind:value={filter}
      />
    {/if}
    {#if !body.binary && body.text !== ''}
      <Button onclick={copy}>
        {copied
          ? i18n.t('response.copied')
          : i18n.t(
              filtered !== null && 'text' in filtered
                ? 'response.copy_result'
                : 'response.copy_body',
            )}
      </Button>
    {/if}
    {#if onsave}
      <Button onclick={onsave}>{i18n.t('response.save_body')}</Button>
    {/if}
  </div>
  {#if filtered !== null && 'problem' in filtered}
    <p class="response-body__note" role="status">{i18n.text(filtered.problem)}</p>
  {/if}
  {#if body.binary}
    <p class="response-body__note">
      {i18n.t('response.body.binary', { size: formatBytes(sizeBytes) })}
    </p>
  {:else if body.text === ''}
    <p class="response-body__note">{i18n.t('response.body.none')}</p>
  {:else}
    {#if body.truncated}
      <p class="response-body__note">
        {i18n.t('response.body.truncated', { size: formatBytes(body.text.length) })}
      </p>
    {/if}
    {#if fitsCodeView(shown)}
      <div class="response-body__text response-body__text--code">
        <CodeViewer
          text={shown}
          isJson={body.pretty}
          label={i18n.t(
            filtered !== null && 'text' in filtered ? 'response.filter_result' : 'response.body',
          )}
        />
      </div>
    {:else}
      <pre class="response-body__text">{shown}</pre>
    {/if}
  {/if}
</div>

<style>
  .response-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .response-body__tools {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
    padding: var(--space-s) var(--space-m) 0;
  }

  .response-body__filter {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
  }

  .response-body__note {
    margin: 0;
    padding: 0 var(--space-l);
    color: var(--color-muted);
  }

  .response-body__text {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: var(--space-l);
    overflow: auto;
    background: var(--color-code-bg);
    font-family: var(--font-mono);
    font-size: var(--font-size-m);
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  /* The code viewer scrolls itself and fills the space. */
  .response-body__text--code {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
  }
</style>
