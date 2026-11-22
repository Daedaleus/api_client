<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import Alert from './Alert.svelte';
  import VariableField from './VariableField.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { METHODS } from '../../lib/http';
  import type { UserText } from '../../lib/types';
  import type { VariableInfo } from '../../lib/variables';

  interface Props {
    method: string;
    url: string;
    sending: boolean;
    error: UserText | null;
    onsend: () => void;
    /** Closes the error message. */
    ondismisserror?: (() => void) | undefined;
    /** Variables for suggestions after `{{` and the tooltip. */
    variables?: ReadonlyMap<string, VariableInfo>;
  }

  let {
    method = $bindable(),
    url = $bindable(),
    sending,
    error,
    onsend,
    ondismisserror,
    variables = new Map(),
  }: Props = $props();

  // A saved request may use another method (e.g. PROPFIND): offer it too.
  let methods = $derived<readonly string[]>(
    (METHODS as readonly string[]).includes(method) ? METHODS : [...METHODS, method],
  );

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
        {#each methods as option (option)}
          <option value={option}>{option}</option>
        {/each}
      </select>
    </label>
    <label class="url-bar__url">
      <span class="url-bar__label">{i18n.t('request.url')}</span>
      <VariableField
        class="url-bar__input"
        label={i18n.t('request.url')}
        placeholder={i18n.t('request.url.placeholder')}
        {variables}
        invalid={error !== null}
        bind:value={url}
      />
    </label>
    <Button
      type="submit"
      variant="primary"
      disabled={sending}
      title={i18n.t('shortcut.hint', {
        action: i18n.t('request.send'),
        keys: i18n.t('shortcut.send'),
      })}
    >
      {sending ? i18n.t('request.sending') : i18n.t('request.send')}
    </Button>
  </div>
  {#if error}
    <Alert text={error} ondismiss={ondismisserror} />
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

  /* The input lives in VariableField; the class is passed down. */
  .url-bar__select,
  .url-bar :global(.url-bar__input) {
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

  .url-bar :global(.url-bar__input) {
    width: 100%;
    font-family: var(--font-mono);
  }
</style>
