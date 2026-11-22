<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { RedirectsView, TransportDraft } from '../../lib/types';

  interface Props {
    transport: TransportDraft;
    /** For the collection: "inherit" means the default (nothing to inherit from). */
    root?: boolean;
  }

  let { transport = $bindable(), root = false }: Props = $props();

  const REDIRECTS: RedirectsView[] = ['inherit', 'follow', 'stop'];
  let redirectOptions = $derived(
    root ? REDIRECTS.filter((option) => option !== 'follow') : REDIRECTS,
  );

  function redirectLabel(option: RedirectsView): string {
    return i18n.t(root && option === 'inherit' ? 'transport.follow' : `transport.${option}`);
  }

  /** The field shows seconds; empty inherits. */
  function setTimeout(text: string) {
    const seconds = Number.parseInt(text, 10);
    transport.timeoutSeconds = Number.isFinite(seconds) && seconds > 0 ? seconds : null;
  }
</script>

<label class="transport-fields__field">
  <span class="transport-fields__label">{i18n.t('transport.redirects')}</span>
  <select class="transport-fields__control" bind:value={transport.redirects}>
    {#each redirectOptions as option (option)}
      <option value={option}>{redirectLabel(option)}</option>
    {/each}
  </select>
</label>
<label class="transport-fields__field">
  <span class="transport-fields__label">{i18n.t('transport.timeout')}</span>
  <span class="transport-fields__timeout">
    <input
      class="transport-fields__control transport-fields__number"
      type="number"
      min="1"
      step="1"
      inputmode="numeric"
      aria-label={i18n.t('transport.timeout')}
      placeholder={i18n.t(root ? 'transport.timeout_default' : 'transport.timeout_inherit')}
      value={transport.timeoutSeconds ?? ''}
      oninput={(event) => setTimeout(event.currentTarget.value)}
    />
    <span class="transport-fields__unit">{i18n.t('transport.seconds')}</span>
  </span>
</label>

<style>
  .transport-fields__field {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-s);
    margin-top: var(--space-s);
  }

  .transport-fields__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .transport-fields__control {
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
  }

  .transport-fields__timeout {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .transport-fields__number {
    width: 12em;
  }

  .transport-fields__unit {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
