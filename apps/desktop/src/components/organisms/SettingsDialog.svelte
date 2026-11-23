<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import Alert from '../molecules/Alert.svelte';
  import ClientCertificatesField from '../molecules/ClientCertificatesField.svelte';
  import { LOCALES, i18n, isLocale } from '../../lib/i18n/i18n.svelte';
  import type { ProxyView } from '../../lib/types';
  import type { SettingsState } from '../../lib/workspace/settings.svelte';

  let { settings }: { settings: SettingsState } = $props();

  type Mode = ProxyView['mode'];
  const MODES: Mode[] = ['system', 'none', 'manual'];

  function setMode(mode: Mode) {
    const editing = settings.editing;
    if (!editing) return;
    editing.proxy = mode === 'manual' ? { mode, url: '', noProxy: 'localhost' } : { mode };
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void settings.save();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      settings.close();
    }
  }
</script>

{#if settings.editing}
  {@const editing = settings.editing}
  <div class="settings-dialog">
    <div
      class="settings-dialog__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-dialog-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <form class="settings-dialog__form" onsubmit={submit}>
        <h2 id="settings-dialog-title" class="settings-dialog__title">
          {i18n.t('settings.title')}
        </h2>

        <section class="settings-dialog__group" aria-labelledby="settings-language">
          <h3 id="settings-language" class="settings-dialog__heading">
            {i18n.t('settings.language')}
          </h3>
          <label class="settings-dialog__field">
            <span class="settings-dialog__label">{i18n.t('settings.language')}</span>
            <select
              class="settings-dialog__control"
              value={i18n.locale}
              onchange={(event) => {
                const value = event.currentTarget.value;
                if (isLocale(value)) i18n.setLocale(value);
              }}
            >
              {#each LOCALES as locale (locale)}
                <option value={locale}>{i18n.t(`settings.language_${locale}`)}</option>
              {/each}
            </select>
          </label>
        </section>

        <section class="settings-dialog__group" aria-labelledby="settings-network">
          <h3 id="settings-network" class="settings-dialog__heading">
            {i18n.t('settings.network')}
          </h3>
          <p class="settings-dialog__note">{i18n.t('settings.network_note')}</p>
          <label class="settings-dialog__field">
            <span class="settings-dialog__label">{i18n.t('settings.proxy')}</span>
            <select
              class="settings-dialog__control"
              value={editing.proxy.mode}
              onchange={(event) => setMode(event.currentTarget.value as Mode)}
            >
              {#each MODES as mode (mode)}
                <option value={mode}>{i18n.t(`settings.proxy_${mode}`)}</option>
              {/each}
            </select>
          </label>
          {#if editing.proxy.mode === 'manual'}
            <label class="settings-dialog__field">
              <span class="settings-dialog__label">{i18n.t('settings.proxy_url')}</span>
              <input
                class="settings-dialog__control"
                type="text"
                spellcheck="false"
                placeholder="http://proxy.example.com:8080"
                bind:value={editing.proxy.url}
              />
            </label>
            <label class="settings-dialog__field">
              <span class="settings-dialog__label">{i18n.t('settings.no_proxy')}</span>
              <input
                class="settings-dialog__control"
                type="text"
                spellcheck="false"
                placeholder="localhost,.internal"
                bind:value={editing.proxy.noProxy}
              />
            </label>
          {/if}

          <div class="settings-dialog__field settings-dialog__field--top">
            <span class="settings-dialog__label">{i18n.t('settings.ca')}</span>
            <div>
              {#if editing.caCertificates.length === 0}
                <p class="settings-dialog__note">{i18n.t('settings.ca_none')}</p>
              {:else}
                <ul class="settings-dialog__files">
                  {#each editing.caCertificates as file, index (file)}
                    <li class="settings-dialog__file">
                      <span class="settings-dialog__path" title={file}>{file}</span>
                      <IconButton
                        icon="trash"
                        label={i18n.t('settings.ca_remove', { file })}
                        onclick={() => settings.removeCertificate(index)}
                      />
                    </li>
                  {/each}
                </ul>
              {/if}
              <Button onclick={() => settings.addCertificate()}>{i18n.t('settings.ca_add')}</Button>
            </div>
          </div>
        </section>

        <section class="settings-dialog__group" aria-labelledby="settings-client">
          <h3 id="settings-client" class="settings-dialog__heading">
            {i18n.t('settings.client')}
          </h3>
          <ClientCertificatesField certificates={editing.clientCertificates} {settings} />
        </section>

        {#if settings.error}
          <Alert text={settings.error} ondismiss={() => (settings.error = null)} />
        {/if}
        <div class="settings-dialog__actions">
          <Button onclick={() => settings.close()}>{i18n.t('dialog.cancel')}</Button>
          <Button type="submit" variant="primary" disabled={settings.saving}>
            {settings.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .settings-dialog {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .settings-dialog__dialog {
    width: min(620px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    overflow-y: auto;
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .settings-dialog__form {
    display: flex;
    flex-direction: column;
    gap: var(--space-l);
  }

  .settings-dialog__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .settings-dialog__group {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .settings-dialog__heading {
    margin: 0;
    font-size: var(--font-size-s);
    font-weight: 600;
    color: var(--color-muted);
  }

  .settings-dialog__note {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .settings-dialog__field {
    display: grid;
    grid-template-columns: 160px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-s);
  }

  .settings-dialog__field--top {
    align-items: start;
  }

  .settings-dialog__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .settings-dialog__control {
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
  }

  .settings-dialog__files {
    margin: 0 0 var(--space-s);
    padding: 0;
    list-style: none;
  }

  .settings-dialog__file {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .settings-dialog__path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
  }

  .settings-dialog__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
