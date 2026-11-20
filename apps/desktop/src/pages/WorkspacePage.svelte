<script lang="ts">
  import AppHeader from '../components/organisms/AppHeader.svelte';
  import DialogHost from '../components/organisms/DialogHost.svelte';
  import RequestEditor from '../components/organisms/RequestEditor.svelte';
  import ResponseViewer from '../components/organisms/ResponseViewer.svelte';
  import Sidebar from '../components/organisms/Sidebar.svelte';
  import TabBar from '../components/organisms/TabBar.svelte';
  import type { Api } from '../lib/api';
  import { Workspace } from '../lib/workspace/workspace.svelte';

  interface Props {
    api: Api;
    /** Remembers the open collections for the next start. */
    storage: Pick<Storage, 'getItem' | 'setItem'> | null;
  }

  let { api, storage }: Props = $props();

  // The page lives as long as the window; api and storage do not change.
  // svelte-ignore state_referenced_locally
  const workspace = new Workspace(api, storage);

  $effect(() => {
    let unsubscribe: (() => void) | undefined;
    let closed = false;
    void workspace.connect().then((stop) => {
      if (closed) stop();
      else unsubscribe = stop;
    });
    void workspace.restore();
    return () => {
      closed = true;
      unsubscribe?.();
    };
  });

  function keydown(event: KeyboardEvent) {
    const tab = workspace.tabs.active;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's' && tab?.saved) {
      event.preventDefault();
      void workspace.save(tab);
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="workspace-page">
  <AppHeader />
  <div class="workspace-page__body">
    <Sidebar {workspace} />
    <main class="workspace-page__main">
      <TabBar {workspace} />
      {#if workspace.tabs.active}
        {@const tab = workspace.tabs.active}
        <div class="workspace-page__editor">
          <RequestEditor
            {tab}
            onsend={() => workspace.tabs.send(tab)}
            onsave={() => workspace.save(tab)}
          />
          <ResponseViewer outcome={tab.outcome} sending={tab.sending} />
        </div>
      {/if}
    </main>
  </div>
  <DialogHost dialogs={workspace.dialogs} />
</div>

<style>
  .workspace-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .workspace-page__body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
  }

  .workspace-page__main {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .workspace-page__editor {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-l);
    padding: var(--space-l);
  }
</style>
