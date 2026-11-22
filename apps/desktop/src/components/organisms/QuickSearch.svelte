<script lang="ts">
  import MethodBadge from '../atoms/MethodBadge.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { searchRequests, type SearchHit } from '../../lib/search';
  import type { Workspace } from '../../lib/workspace/workspace.svelte';

  let { workspace }: { workspace: Workspace } = $props();

  let query = $state('');
  let selected = $state(0);
  let input = $state<HTMLInputElement | null>(null);

  let hits = $derived(searchRequests(workspace.collections.collections, query));

  // A new search starts empty, with the field focused.
  $effect(() => {
    if (workspace.searching) {
      query = '';
      queueMicrotask(() => input?.focus());
    }
  });

  // The selection stays on a hit while the list changes.
  $effect(() => {
    if (selected >= hits.length) selected = Math.max(0, hits.length - 1);
  });

  function open(hit: SearchHit) {
    workspace.searching = false;
    void workspace.openRequest(hit.collectionId, hit.request.id);
  }

  function keydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'ArrowDown':
        selected = Math.min(selected + 1, hits.length - 1);
        break;
      case 'ArrowUp':
        selected = Math.max(selected - 1, 0);
        break;
      case 'Enter': {
        const hit = hits[selected];
        if (hit) open(hit);
        break;
      }
      case 'Escape':
        workspace.searching = false;
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

{#if workspace.searching}
  <div class="quick-search" role="presentation" onclick={() => (workspace.searching = false)}>
    <div
      class="quick-search__dialog"
      role="dialog"
      aria-modal="true"
      aria-label={i18n.t('search.title')}
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={keydown}
    >
      <input
        class="quick-search__input"
        type="search"
        spellcheck="false"
        autocomplete="off"
        role="combobox"
        aria-expanded={hits.length > 0}
        aria-controls="quick-search-results"
        aria-activedescendant={hits.length > 0 ? `quick-search-hit-${selected}` : undefined}
        aria-label={i18n.t('search.title')}
        placeholder={i18n.t('search.placeholder')}
        bind:value={query}
        bind:this={input}
        oninput={() => (selected = 0)}
      />
      {#if query.trim() !== '' && hits.length === 0}
        <p class="quick-search__empty">{i18n.t('search.none')}</p>
      {/if}
      <ul id="quick-search-results" class="quick-search__results" role="listbox">
        {#each hits as hit, index (hit.request.id)}
          <li
            id="quick-search-hit-{index}"
            class="quick-search__hit"
            class:quick-search__hit--selected={index === selected}
            role="option"
            aria-selected={index === selected}
            onclick={() => open(hit)}
            onkeydown={() => {}}
            onpointermove={() => (selected = index)}
          >
            <MethodBadge method={hit.request.method} />
            <span class="quick-search__name">{hit.request.name}</span>
            <span class="quick-search__path">{hit.path}</span>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .quick-search {
    position: fixed;
    inset: 0;
    z-index: 6;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: rgba(0, 0, 0, 0.35);
  }

  .quick-search__dialog {
    width: min(560px, calc(100% - 2 * var(--space-l)));
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
    padding: var(--space-m);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .quick-search__input {
    height: var(--control-height);
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
    font: inherit;
  }

  .quick-search__empty {
    margin: 0;
    color: var(--color-muted);
  }

  .quick-search__results {
    max-height: 50vh;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  .quick-search__hit {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px var(--space-s);
    align-items: center;
    padding: var(--space-xs) var(--space-s);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .quick-search__hit--selected {
    background: var(--color-raised);
  }

  .quick-search__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-search__path {
    grid-column: 2;
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }
</style>
