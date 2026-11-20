<script lang="ts">
  import AppHeader from '../components/organisms/AppHeader.svelte';
  import ResponseViewer from '../components/organisms/ResponseViewer.svelte';
  import UrlBar from '../components/molecules/UrlBar.svelte';
  import type { Api } from '../lib/api';
  import { RequestController } from '../lib/request.svelte';

  let { api }: { api: Api } = $props();

  // The page lives as long as the window; the api does not change.
  // svelte-ignore state_referenced_locally
  const request = new RequestController(api);

  $effect(() => {
    let unsubscribe: (() => void) | undefined;
    let closed = false;
    void request.connect().then((stop) => {
      if (closed) stop();
      else unsubscribe = stop;
    });
    return () => {
      closed = true;
      unsubscribe?.();
    };
  });
</script>

<div class="request-page">
  <AppHeader />
  <main class="request-page__main">
    <UrlBar
      bind:method={request.method}
      bind:url={request.url}
      sending={request.sending}
      error={request.error}
      onsend={() => request.send()}
    />
    <ResponseViewer outcome={request.outcome} sending={request.sending} />
  </main>
</div>

<style>
  .request-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .request-page__main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-l);
    padding: var(--space-l);
  }
</style>
