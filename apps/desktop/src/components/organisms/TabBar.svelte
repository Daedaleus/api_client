<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import MethodBadge from '../atoms/MethodBadge.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { Workspace } from '../../lib/workspace/workspace.svelte';

  let { workspace }: { workspace: Workspace } = $props();

  let tabs = $derived(workspace.tabs);
</script>

<div class="tab-bar" role="tablist" aria-label={i18n.t('tab.list')}>
  {#each tabs.tabs as tab (tab.key)}
    <div class="tab-bar__tab" class:tab-bar__tab--active={tab.key === tabs.activeKey}>
      <button
        type="button"
        role="tab"
        class="tab-bar__select"
        aria-selected={tab.key === tabs.activeKey}
        onclick={() => tabs.activate(tab.key)}
      >
        <MethodBadge method={tab.draft.method} />
        <span class="tab-bar__name">{tab.draft.name || i18n.t('tab.untitled')}</span>
        {#if tab.dirty}
          <span class="tab-bar__dirty" role="img" aria-label={i18n.t('tab.unsaved')}></span>
        {/if}
      </button>
      <IconButton
        icon="close"
        label={i18n.t('tab.close')}
        onclick={() => workspace.closeTab(tab)}
      />
    </div>
  {/each}
  <IconButton icon="plus" label={i18n.t('tab.new')} onclick={() => tabs.openScratch()} />
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    min-height: 38px;
    overflow-x: auto;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
  }

  .tab-bar__tab {
    display: flex;
    align-items: center;
    padding-right: var(--space-xs);
    border-right: 1px solid var(--color-border);
  }

  .tab-bar__tab--active {
    background: var(--color-bg);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .tab-bar__select {
    display: flex;
    align-items: center;
    gap: var(--space-s);
    padding: 0 var(--space-s) 0 var(--space-m);
    border: 0;
    background: transparent;
    white-space: nowrap;
    cursor: pointer;
  }

  .tab-bar__name {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-bar__dirty {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-accent);
  }
</style>
