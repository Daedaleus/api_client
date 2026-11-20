<script lang="ts">
  import { i18n } from './lib/i18n/i18n.svelte';
  import { tauriApi } from './lib/tauri';
  import RequestPage from './pages/RequestPage.svelte';

  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
</script>

<RequestPage api={tauriApi} />
