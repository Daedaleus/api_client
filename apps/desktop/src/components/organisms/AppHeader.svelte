<script lang="ts">
  import { LOCALES, type Locale, i18n } from '../../lib/i18n/i18n.svelte';

  function change(event: Event) {
    i18n.setLocale((event.currentTarget as HTMLSelectElement).value as Locale);
  }
</script>

<header class="app-header">
  <span class="app-header__title">{i18n.t('app.title')}</span>
  <label class="app-header__language">
    <span>{i18n.t('settings.language')}</span>
    <select class="app-header__select" value={i18n.locale} onchange={change}>
      {#each LOCALES as locale (locale)}
        <option value={locale}>{i18n.t(`settings.language.${locale}`)}</option>
      {/each}
    </select>
  </label>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-m);
    padding: var(--space-s) var(--space-l);
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
  }

  .app-header__title {
    font-weight: 600;
    font-size: var(--font-size-l);
  }

  .app-header__language {
    display: flex;
    align-items: center;
    gap: var(--space-s);
    color: var(--color-muted);
  }

  .app-header__select {
    height: 30px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-raised);
    padding: 0 var(--space-s);
  }
</style>
