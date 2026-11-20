<script lang="ts">
  import Icon from '../atoms/Icon.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import MethodBadge from '../atoms/MethodBadge.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { TreeItemView } from '../../lib/types';
  import type { Workspace } from '../../lib/workspace/workspace.svelte';
  import TreeNode from './TreeNode.svelte';

  interface Props {
    item: TreeItemView;
    collectionId: string;
    workspace: Workspace;
    depth: number;
  }

  let { item, collectionId, workspace, depth }: Props = $props();

  let expanded = $state(true);
  let active = $derived(
    item.kind === 'request' && workspace.tabs.active?.isFor(collectionId, item.id) === true,
  );
</script>

<li class="tree-node">
  <div class="tree-node__row" class:tree-node__row--active={active} style:--depth={depth}>
    {#if item.kind === 'folder'}
      <button
        type="button"
        class="tree-node__main"
        aria-expanded={expanded}
        onclick={() => (expanded = !expanded)}
      >
        <span class="tree-node__chevron" class:tree-node__chevron--open={expanded}>
          <Icon name="chevron" size={12} />
        </span>
        <span class="tree-node__name">{item.name}</span>
      </button>
    {:else}
      <button
        type="button"
        class="tree-node__main"
        aria-current={active ? 'page' : undefined}
        onclick={() => workspace.openRequest(collectionId, item.id)}
      >
        <MethodBadge method={item.method} />
        <span class="tree-node__name">{item.name}</span>
      </button>
    {/if}
    {#if item.readOnly}
      <span class="tree-node__note">{i18n.t('collection.read_only')}</span>
    {/if}
    <span class="tree-node__actions">
      {#if item.kind === 'folder'}
        <IconButton
          icon="plus"
          label={i18n.t('collection.new_request')}
          onclick={() => workspace.newRequest(collectionId, item.id)}
        />
        <IconButton
          icon="folder-plus"
          label={i18n.t('collection.new_folder')}
          onclick={() => workspace.newFolder(collectionId, item.id)}
        />
        <IconButton
          icon="braces"
          label={i18n.t('collection.folder_variables')}
          onclick={() => workspace.variables.open(collectionId, item.id)}
        />
      {/if}
      <IconButton
        icon="edit"
        label={i18n.t('collection.rename')}
        onclick={() => workspace.rename(collectionId, item)}
      />
      <IconButton
        icon="trash"
        label={i18n.t('collection.delete')}
        onclick={() => workspace.remove(collectionId, item)}
      />
    </span>
  </div>
  {#if item.kind === 'folder' && expanded && item.items.length > 0}
    <ul class="tree-node__children">
      {#each item.items as child (child.id)}
        <TreeNode item={child} {collectionId} {workspace} depth={depth + 1} />
      {/each}
    </ul>
  {/if}
</li>

<style>
  .tree-node {
    list-style: none;
  }

  .tree-node__children {
    margin: 0;
    padding: 0;
  }

  .tree-node__row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding-left: calc(var(--depth) * 14px);
    border-radius: 6px;
  }

  .tree-node__row:hover,
  .tree-node__row--active {
    background: var(--color-raised);
  }

  .tree-node__main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-s);
    padding: 5px var(--space-s);
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .tree-node__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tree-node__chevron {
    display: inline-flex;
    color: var(--color-muted);
    transition: transform 0.1s;
  }

  .tree-node__chevron--open {
    transform: rotate(90deg);
  }

  .tree-node__note {
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }

  .tree-node__actions {
    display: flex;
    opacity: 0;
  }

  .tree-node__row:hover .tree-node__actions,
  .tree-node__row:focus-within .tree-node__actions {
    opacity: 1;
  }
</style>
