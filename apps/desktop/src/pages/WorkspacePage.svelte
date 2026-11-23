<script lang="ts">
  import AppHeader from '../components/organisms/AppHeader.svelte';
  import EnvironmentPicker from '../components/molecules/EnvironmentPicker.svelte';
  import DialogHost from '../components/organisms/DialogHost.svelte';
  import EnvironmentEditor from '../components/organisms/EnvironmentEditor.svelte';
  import RecipientsDialog from '../components/organisms/RecipientsDialog.svelte';
  import CookiesDialog from '../components/organisms/CookiesDialog.svelte';
  import CurlImportDialog from '../components/organisms/CurlImportDialog.svelte';
  import DefaultsDialog from '../components/organisms/DefaultsDialog.svelte';
  import QuickSearch from '../components/organisms/QuickSearch.svelte';
  import SettingsDialog from '../components/organisms/SettingsDialog.svelte';
  import RequestEditor from '../components/organisms/RequestEditor.svelte';
  import ResponseViewer from '../components/organisms/ResponseViewer.svelte';
  import Sidebar from '../components/organisms/Sidebar.svelte';
  import TabBar from '../components/organisms/TabBar.svelte';
  import type { Api } from '../lib/api';
  import { shortcutFor } from '../lib/shortcuts';
  import { Workspace } from '../lib/workspace/workspace.svelte';

  interface Props {
    api: Api;
    /** Remembers open collections and chosen environments for the next start. */
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

  // Remember the open tabs – a moment after the last change, not per key.
  const REMEMBER_DELAY = 500;
  $effect(() => {
    // Read every tab's draft so typing counts as a change.
    JSON.stringify(workspace.tabs.tabs.map((tab) => [tab.saved, tab.draft, tab.dirty]));
    void workspace.tabs.activeKey;
    const timer = setTimeout(() => workspace.rememberTabs(), REMEMBER_DELAY);
    return () => clearTimeout(timer);
  });

  function keydown(event: KeyboardEvent) {
    const action = shortcutFor(event);
    if (action === null) return;
    const tab = workspace.tabs.active;
    switch (action) {
      case 'save':
        if (!tab?.saved) return;
        void workspace.save(tab);
        break;
      case 'newRequest':
        workspace.tabs.openScratch();
        break;
      case 'send':
        if (!tab || tab.sending) return;
        void workspace.send(tab);
        break;
      case 'search':
        workspace.searching = true;
        break;
      case 'environment': {
        // Only saved requests of a collection have an environment picker.
        const picker = document.querySelector<HTMLSelectElement>('.environment-picker__select');
        if (!picker) return;
        picker.focus();
        break;
      }
    }
    event.preventDefault();
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
        {@const collection = workspace.collectionOf(tab)}
        {@const chosen = workspace.environments.of(collection)}
        <div class="workspace-page__editor">
          <RequestEditor
            {tab}
            assist={workspace.scriptAssist}
            onsend={() => workspace.send(tab)}
            onsave={() => workspace.save(tab)}
            variables={workspace.variablesFor(tab)}
            variableInfo={workspace.variableInfo(tab)}
            onpickfile={() => workspace.pickFile()}
            checksCertificates={workspace.checksCertificates(tab)}
            oncopycurl={() => workspace.curlFor(tab)}
            onexamples={() => workspace.examples.load(tab)}
            ondeleteexample={(index) => workspace.examples.remove(tab, index)}
            oncopysnippet={(language) => workspace.snippetFor(tab, language)}
          >
            {#snippet environment()}
              {#if collection}
                <EnvironmentPicker
                  environments={collection.environments}
                  chosen={chosen?.id ?? null}
                  onchoose={(id) => workspace.environments.choose(collection.id, id)}
                  onedit={(id) => workspace.editEnvironment(collection.id, id)}
                  onnew={() => workspace.newEnvironment(collection.id)}
                  onrecipients={() => workspace.recipients.open(collection.id)}
                />
              {/if}
            {/snippet}
          </RequestEditor>
          <ResponseViewer
            outcome={tab.outcome}
            sending={tab.sending}
            onsavebody={tab.outcomeRun === null ? undefined : () => workspace.saveResponse(tab)}
            onsaveexample={workspace.examples.canSave(tab)
              ? () => workspace.examples.saveFrom(tab)
              : undefined}
          />
        </div>
      {/if}
    </main>
  </div>
  {#if workspace.editing}
    <EnvironmentEditor
      draft={workspace.editing.draft}
      error={workspace.collections.error}
      onsave={() => workspace.saveEnvironment()}
      oncancel={() => (workspace.editing = null)}
      ondelete={() => workspace.deleteEnvironment()}
    />
  {/if}
  <CurlImportDialog curl={workspace.curl} />
  <QuickSearch {workspace} />
  <SettingsDialog settings={workspace.settings} />
  {#if workspace.defaults.editing}
    <DefaultsDialog defaults={workspace.defaults} assist={workspace.scriptAssist} />
  {/if}
  {#if workspace.recipients.editing}
    <RecipientsDialog recipients={workspace.recipients} />
  {/if}
  <CookiesDialog cookies={workspace.cookies} />
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
