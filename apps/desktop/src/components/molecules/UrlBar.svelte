<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { METHODS } from '../../lib/request.svelte';
  import type { UserText } from '../../lib/types';

  interface Props {
    method: string;
    url: string;
    sending: boolean;
    error: UserText | null;
    onsend: () => void;
  }

  let { method = $bindable(), url = $bindable(), sending, error, onsend }: Props = $props();

  function submit(event: SubmitEvent) {
    event.preventDefault();
    onsend();
  }
</script>

<form class="url-bar" onsubmit={submit}>
  <div class="url-bar__row">
    <label class="url-bar__method">
      <span class="url-bar__label">{i18n.t('request.method')}</span>
      <select class="url-bar__select" bind:value={method}>
        {#each METHODS as option (option)}
          <option value={option}>{option}</option>
        {/each}
      </select>
    </label>
    <label class="url-bar__url">
      <span class="url-bar__label">{i18n.t('request.url')}</span>
      <input
        class="url-bar__input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder={i18n.t('request.url.placeholder')}
        aria-invalid={error !== null}
        bind:value={url}
      />
    </label>
    <Button type="submit" variant="primary" disabled={sending}>
      {sending ? i18n.t('request.sending') : i18n.t('request.send')}
    </Button>
  </div>
  {#if error}
    <p class="url-bar__error" role="alert">{i18n.text(error)}</p>
  {/if}
</form>

<style>
  .url-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .url-bar__row {
    display: flex;
    gap: var(--space-s);
    align-items: center;
  }

  .url-bar__label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .url-bar__select,
  .url-bar__input {
    height: var(--control-height);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-surface);
    padding: 0 var(--space-m);
  }

  .url-bar__select {
    font-family: var(--font-mono);
    font-weight: 700;
  }

  .url-bar__url {
    flex: 1;
    min-width: 0;
  }

  .url-bar__input {
    width: 100%;
    font-family: var(--font-mono);
  }

  .url-bar__error {
    margin: 0;
    color: var(--color-error);
  }
</style>
