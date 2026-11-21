<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import ContextMenu from '../molecules/ContextMenu.svelte';
  import MenuButton from '../molecules/MenuButton.svelte';
  import TreeNode from '../molecules/TreeNode.svelte';
  import { collectionMenu, newMenu } from '../../lib/workspace/menus';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { Workspace } from '../../lib/workspace/workspace.svelte';

  let { workspace }: { workspace: Workspace } = $props();

  let collections = $derived(workspace.collections.collections);
</script>

<aside class="sidebar" aria-label={i18n.t('sidebar.title')}>
  <div class="sidebar__toolbar">
    <span class="sidebar__title">{i18n.t('sidebar.title')}</span>
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
  </div>

  {#if workspace.collections.error}
    <p class="sidebar__error" role="alert">{i18n.text(workspace.collections.error)}</p>
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

  .sidebar__title {
    flex: 1;
    font-size: var(--font-size-s);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-muted);
  }

  .sidebar__error,
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

  .sidebar__tree {
    margin: 0;
    padding: 0;
  }
</style>
