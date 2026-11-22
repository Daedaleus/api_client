<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { UserText } from '../../lib/types';

  interface Props {
    text: UserText;
    /** Removes the message; without it the message cannot be closed. */
    ondismiss?: (() => void) | undefined;
  }

  let { text, ondismiss }: Props = $props();
</script>

<div class="alert" role="alert">
  <span class="alert__text">{i18n.text(text)}</span>
  {#if ondismiss}
    <IconButton icon="close" label={i18n.t('alert.dismiss')} onclick={ondismiss} />
  {/if}
</div>

<style>
  .alert {
    display: flex;
    align-items: flex-start;
    gap: var(--space-s);
    padding: var(--space-s) var(--space-m);
    border-radius: var(--radius);
    background: var(--color-error-bg);
    color: var(--color-error);
    font-size: var(--font-size-s);
  }

  .alert__text {
    flex: 1;
    min-width: 0;
    padding-top: 4px;
    overflow-wrap: anywhere;
  }
</style>
