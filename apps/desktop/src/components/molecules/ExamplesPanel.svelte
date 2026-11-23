<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { ExampleView } from '../../lib/types';
  import ResponseBody from './ResponseBody.svelte';

  interface Props {
    /** `null` while loading. */
    examples: ExampleView[] | null;
    ondelete: (index: number) => void;
  }

  let { examples, ondelete }: Props = $props();

  let selected = $state(0);
  let shown = $derived(examples?.[Math.min(selected, (examples?.length ?? 1) - 1)] ?? null);
</script>

<div class="examples-panel">
  {#if examples === null}
    <p class="examples-panel__note">{i18n.t('examples.loading')}</p>
  {:else if examples.length === 0}
    <p class="examples-panel__note">{i18n.t('examples.none')}</p>
  {:else}
    <ul class="examples-panel__list" aria-label={i18n.t('examples.list')}>
      {#each examples as example, index (index)}
        <li class="examples-panel__item">
          <button
            type="button"
            class="examples-panel__pick"
            class:examples-panel__pick--active={shown === example}
            aria-pressed={shown === example}
            onclick={() => (selected = index)}
          >
            <span class="examples-panel__status">{example.response.status}</span>
            {example.name}
          </button>
          <IconButton
            icon="trash"
            label={i18n.t('examples.delete', { name: example.name })}
            onclick={() => ondelete(index)}
          />
        </li>
      {/each}
    </ul>
    {#if shown}
      <div class="examples-panel__detail">
        <p class="examples-panel__meta">
          <strong>{shown.response.status} {shown.response.reason ?? ''}</strong>
          {#each shown.response.headers as header, index (index)}
            <span class="examples-panel__header">{header.name}: {header.value}</span>
          {/each}
        </p>
        <ResponseBody body={shown.response.body} sizeBytes={shown.response.sizeBytes} />
      </div>
    {/if}
  {/if}
</div>

<style>
  .examples-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
    min-height: 0;
  }

  .examples-panel__note {
    margin: 0;
    color: var(--color-muted);
  }

  .examples-panel__list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .examples-panel__item {
    display: flex;
    align-items: center;
  }

  .examples-panel__pick {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 2px var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
  }

  .examples-panel__pick--active {
    background: var(--color-raised);
    border-color: var(--color-accent);
  }

  .examples-panel__status {
    font-family: var(--font-mono);
    font-weight: 700;
  }

  .examples-panel__detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
    min-height: 240px;
  }

  .examples-panel__meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs) var(--space-m);
    margin: 0;
    font-size: var(--font-size-s);
  }

  .examples-panel__header {
    color: var(--color-muted);
    font-family: var(--font-mono);
  }
</style>
