<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { CertificateCheckView } from '../../lib/types';

  interface Props {
    check: CertificateCheckView;
    /** For the collection: "inherit" means "check" (nothing to inherit from). */
    root?: boolean;
  }

  let { check = $bindable(), root = false }: Props = $props();

  const OPTIONS: CertificateCheckView[] = ['inherit', 'on', 'off'];
  let options = $derived(root ? OPTIONS.filter((option) => option !== 'on') : OPTIONS);

  function label(option: CertificateCheckView): string {
    return i18n.t(root && option === 'inherit' ? 'tls.on' : `tls.${option}`);
  }
</script>

<label class="certificate-check-field">
  <span class="certificate-check-field__label">{i18n.t('tls.label')}</span>
  <select class="certificate-check-field__select" bind:value={check}>
    {#each options as option (option)}
      <option value={option}>{label(option)}</option>
    {/each}
  </select>
</label>
{#if check === 'off'}
  <p class="certificate-check-field__warning">{i18n.t('tls.off_note')}</p>
{/if}

<style>
  .certificate-check-field {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-s);
  }

  .certificate-check-field__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .certificate-check-field__select {
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
  }

  .certificate-check-field__warning {
    margin: var(--space-s) 0 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }
</style>
