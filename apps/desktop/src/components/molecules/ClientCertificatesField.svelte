<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { SettingsState } from '../../lib/workspace/settings.svelte';
  import type { ClientCertificateView } from '../../lib/types';

  interface Props {
    certificates: ClientCertificateView[];
    settings: SettingsState;
  }

  let { certificates, settings }: Props = $props();

  /** Whether a passphrase is or will be stored. */
  function hasPassphrase(entry: ClientCertificateView): boolean {
    return (
      entry.newPassphrase !== '' &&
      (entry.encryptedPassphrase !== null || entry.newPassphrase !== null)
    );
  }
</script>

<div class="client-certificates">
  <p class="client-certificates__note">{i18n.t('settings.client_note')}</p>
  {#each certificates as entry, index (index)}
    <fieldset class="client-certificates__entry">
      <legend class="client-certificates__legend">
        {entry.host.trim() || i18n.t('settings.client_new')}
      </legend>
      <label class="client-certificates__field">
        <span class="client-certificates__label">{i18n.t('settings.client_host')}</span>
        <input
          class="client-certificates__control"
          type="text"
          spellcheck="false"
          placeholder="api.internal, *.internal, api.internal:8443"
          bind:value={entry.host}
        />
      </label>
      <label class="client-certificates__field">
        <span class="client-certificates__label">{i18n.t('settings.client_certificate')}</span>
        <span class="client-certificates__row">
          <input
            class="client-certificates__control"
            type="text"
            spellcheck="false"
            bind:value={entry.certificate}
          />
          <Button onclick={() => settings.pickClientFile(index, 'certificate')}>
            {i18n.t('settings.client_choose')}
          </Button>
        </span>
      </label>
      <label class="client-certificates__field">
        <span class="client-certificates__label">{i18n.t('settings.client_key')}</span>
        <span class="client-certificates__row">
          <input
            class="client-certificates__control"
            type="text"
            spellcheck="false"
            placeholder={i18n.t('settings.client_key_placeholder')}
            bind:value={entry.key}
          />
          <Button onclick={() => settings.pickClientFile(index, 'key')}>
            {i18n.t('settings.client_choose')}
          </Button>
        </span>
      </label>
      <label class="client-certificates__field">
        <span class="client-certificates__label">{i18n.t('settings.client_passphrase')}</span>
        <span class="client-certificates__row">
          <input
            class="client-certificates__control"
            type="password"
            autocomplete="off"
            placeholder={i18n.t(
              entry.encryptedPassphrase === null
                ? 'settings.client_passphrase_none'
                : 'settings.client_passphrase_stored',
            )}
            value={entry.newPassphrase ?? ''}
            oninput={(event) => settings.typePassphrase(index, event.currentTarget.value)}
          />
          {#if hasPassphrase(entry)}
            <Button onclick={() => settings.removePassphrase(index)}>
              {i18n.t('settings.client_passphrase_remove')}
            </Button>
          {/if}
        </span>
      </label>
      <div class="client-certificates__actions">
        <IconButton
          icon="trash"
          label={i18n.t('settings.client_remove', { host: entry.host.trim() || '…' })}
          onclick={() => settings.removeClientCertificate(index)}
        />
      </div>
    </fieldset>
  {/each}
  <div>
    <Button onclick={() => settings.addClientCertificate()}>{i18n.t('settings.client_add')}</Button>
  </div>
</div>

<style>
  .client-certificates {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .client-certificates__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
    line-height: 1.5;
  }

  .client-certificates__entry {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    margin: 0;
    padding: var(--space-s) var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .client-certificates__legend {
    padding: 0 var(--space-xs);
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .client-certificates__field {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-s);
  }

  .client-certificates__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .client-certificates__row {
    display: flex;
    gap: var(--space-s);
  }

  .client-certificates__control {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
  }

  .client-certificates__actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
