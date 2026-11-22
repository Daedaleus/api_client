<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import MethodBadge from '../atoms/MethodBadge.svelte';
  import Alert from '../molecules/Alert.svelte';
  import { formatDuration } from '../../lib/format';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { HistoryState } from '../../lib/workspace/history.svelte';

  let { history }: { history: HistoryState } = $props();

  let days = $derived(history.days);

  function dayLabel(day: number): string {
    return new Date(day).toLocaleDateString(i18n.locale, { dateStyle: 'full' });
  }

  function timeLabel(atMs: number): string {
    return new Date(atMs).toLocaleTimeString(i18n.locale, { timeStyle: 'short' });
  }
</script>

<div class="history-list">
  <div class="history-list__toolbar">
    <input
      class="history-list__filter"
      type="search"
      spellcheck="false"
      aria-label={i18n.t('history.filter')}
      placeholder={i18n.t('history.filter')}
      bind:value={history.filter}
    />
    <IconButton icon="trash" label={i18n.t('history.clear')} onclick={() => history.clear()} />
  </div>

  {#if history.error}
    <Alert text={history.error} ondismiss={() => (history.error = null)} />
  {/if}

  {#if history.entries.length === 0}
    <p class="history-list__empty">{i18n.t('history.empty')}</p>
  {:else if days.length === 0}
    <p class="history-list__empty">{i18n.t('history.no_match')}</p>
  {/if}

  {#each days as day (day.day)}
    <section class="history-list__day" aria-label={dayLabel(day.day)}>
      <h3 class="history-list__day-title">{dayLabel(day.day)}</h3>
      <ul class="history-list__entries">
        {#each day.entries as entry (entry.id)}
          <li>
            <button
              type="button"
              class="history-list__entry"
              aria-label={i18n.t('history.open', { method: entry.method, url: entry.url })}
              title={entry.url}
              onclick={() => history.open(entry.id)}
            >
              <MethodBadge method={entry.method} />
              <span class="history-list__url">{entry.url}</span>
              <span
                class="history-list__status"
                class:history-list__status--failed={entry.status === null || entry.status >= 400}
              >
                {entry.status ?? i18n.t('history.failed')}
              </span>
              <span class="history-list__meta">
                {timeLabel(entry.atMs)}{entry.elapsedMs === null
                  ? ''
                  : ` · ${formatDuration(entry.elapsedMs)}`}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>

<style>
  .history-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .history-list__toolbar {
    display: flex;
    gap: var(--space-xs);
  }

  .history-list__filter {
    flex: 1;
    min-width: 0;
    height: var(--control-height);
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
    font: inherit;
  }

  .history-list__empty {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .history-list__day-title {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-s);
    font-weight: 600;
    color: var(--color-muted);
  }

  .history-list__entries {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .history-list__entry {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 2px var(--space-s);
    width: 100%;
    padding: var(--space-xs) var(--space-s);
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .history-list__entry:hover,
  .history-list__entry:focus-visible {
    background: var(--color-raised);
  }

  .history-list__url {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-list__status {
    font-size: var(--font-size-s);
    color: var(--color-success);
  }

  .history-list__status--failed {
    color: var(--color-error);
  }

  .history-list__meta {
    grid-column: 2 / 4;
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }
</style>
