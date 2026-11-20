<script lang="ts">
  import { i18n } from './lib/i18n/i18n.svelte';
  import { tauriApi } from './lib/tauri';
  import WorkspacePage from './pages/WorkspacePage.svelte';

  function browserStorage(): Storage | null {
    try {
      return window.localStorage;
    } catch {
      return null;
    }
  }

  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
</script>

<WorkspacePage api={tauriApi} storage={browserStorage()} />
