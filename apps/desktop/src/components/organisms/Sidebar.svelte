<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import Alert from '../molecules/Alert.svelte';
  import ContextMenu from '../molecules/ContextMenu.svelte';
  import MenuButton from '../molecules/MenuButton.svelte';
  import TreeNode from '../molecules/TreeNode.svelte';
  import HistoryList from './HistoryList.svelte';
  import { collectionMenu, newMenu } from '../../lib/workspace/menus';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { Workspace } from '../../lib/workspace/workspace.svelte';

  let { workspace }: { workspace: Workspace } = $props();

  let collections = $derived(workspace.collections.collections);
  let showHistory = $derived(workspace.history.visible);
</script>

<aside class="sidebar" aria-label={i18n.t('sidebar.title')}>
  <div class="sidebar__toolbar">
    <div class="sidebar__views" role="tablist" aria-label={i18n.t('sidebar.views')}>
      {#each [false, true] as history (history)}
        <button
          type="button"
          role="tab"
          class="sidebar__view"
          class:sidebar__view--active={showHistory === history}
          aria-selected={showHistory === history}
          onclick={() => workspace.history.show(history)}
        >
          {i18n.t(history ? 'history.title' : 'sidebar.title')}
        </button>
      {/each}
    </div>
    {#if !showHistory}
      <IconButton
        icon="open"
        label={i18n.t('menu.open_collection')}
        onclick={() => workspace.openCollection()}
      />
      <MenuButton
        text={i18n.t('menu.new')}
        label={i18n.t('menu.new_title')}
        items={newMenu(workspace)}
      />
    {/if}
  </div>

  {#if showHistory}
    <HistoryList history={workspace.history} />
  {:else}
    {#if workspace.collections.error}
      <Alert
        text={workspace.collections.error}
        ondismiss={() => (workspace.collections.error = null)}
      />
    {/if}

    {#if collections.length === 0}
      <p class="sidebar__empty">{i18n.t('sidebar.empty')}</p>
    {/if}

    {#each collections as collection (collection.id)}
      <section class="sidebar__collection" aria-label={collection.name}>
        <ContextMenu
          label={i18n.t('menu.more', { name: collection.name })}
          items={() => collectionMenu(workspace, collection)}
        >
          <div class="sidebar__collection-header">
            <span class="sidebar__collection-name" title={collection.root}>{collection.name}</span>
            {#if collection.temporary}
              <button
                type="button"
                class="sidebar__unsaved"
                title={i18n.t('collection.unsaved_hint')}
                onclick={() => workspace.saveCollectionAs(collection.id)}
              >
                {i18n.t('collection.unsaved')}
              </button>
            {/if}
            <IconButton
              icon="plus"
              label={i18n.t('collection.new_request')}
              onclick={() => workspace.newRequest(collection.id, collection.id)}
            />
            <MenuButton
              icon="more"
              label={i18n.t('menu.more', { name: collection.name })}
              items={collectionMenu(workspace, collection)}
            />
          </div>
        </ContextMenu>
        {#if collection.problems.length > 0}
          <div class="sidebar__problems">
            <span>{i18n.t('sidebar.problems', { count: String(collection.problems.length) })}</span>
            <ul>
              {#each collection.problems as problem, index (index)}
                <li>{i18n.text(problem)}</li>
              {/each}
            </ul>
          </div>
        {/if}
        <ul class="sidebar__tree">
          {#each collection.items as item (item.id)}
            <TreeNode {item} collectionId={collection.id} {workspace} depth={0} />
          {/each}
        </ul>
      </section>
    {/each}
  {/if}
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
    padding: var(--space-m);
    overflow-y: auto;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
  }

  .sidebar__toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .sidebar__views {
    flex: 1;
    display: flex;
    gap: var(--space-xs);
  }

  .sidebar__view {
    padding: var(--space-xs) 0;
    margin-right: var(--space-s);
    border: 0;
    background: transparent;
    font-size: var(--font-size-s);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-muted);
    cursor: pointer;
  }

  .sidebar__view--active {
    color: var(--color-text);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .sidebar__problems {
    margin: 0;
    padding: var(--space-s) var(--space-m);
    border-radius: var(--radius);
    background: var(--color-error-bg);
    color: var(--color-error);
    font-size: var(--font-size-s);
  }

  .sidebar__problems ul {
    margin: var(--space-xs) 0 0;
    padding-left: var(--space-l);
  }

  .sidebar__empty {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .sidebar__collection-header {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-s);
  }

  .sidebar__collection-name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar__unsaved {
    flex: none;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-warning);
    border-radius: 9px;
    background: transparent;
    color: var(--color-warning);
    font-size: var(--font-size-s);
    cursor: pointer;
  }

  .sidebar__tree {
    margin: 0;
    padding: 0;
  }
</style>
