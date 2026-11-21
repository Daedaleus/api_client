<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { Dialogs } from '../../lib/workspace/dialogs.svelte';

  let { dialogs }: { dialogs: Dialogs } = $props();

  let value = $state('');
  let input = $state<HTMLInputElement | null>(null);
  let form = $state<HTMLFormElement | null>(null);

  // A name question starts with its suggested value, selected for typing over;
  // a confirmation focuses its OK button. So the keyboard is in the dialog.
  $effect(() => {
    const current = dialogs.current;
    if (current?.kind === 'name') {
      value = current.value;
      queueMicrotask(() => input?.select());
    } else if (current) {
      queueMicrotask(() => form?.querySelector<HTMLButtonElement>('[type=submit]')?.focus());
    }
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const current = dialogs.current;
    if (current?.kind === 'name') current.answer(value);
    else current?.answer(true);
  }

  function cancel() {
    const current = dialogs.current;
    if (current?.kind === 'name') current.answer(null);
    else current?.answer(false);
  }

  // Escape cancels wherever the focus is while a dialog is open.
  function keydown(event: KeyboardEvent) {
    if (dialogs.current && event.key === 'Escape') {
      event.preventDefault();
      cancel();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

{#if dialogs.current}
  {@const current = dialogs.current}
  <div class="dialog-host">
    <div class="dialog-host__dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
      <form class="dialog-host__form" onsubmit={submit} bind:this={form}>
        <h2 id="dialog-title" class="dialog-host__title">{i18n.text(current.title)}</h2>
        {#if current.kind === 'name'}
          <label class="dialog-host__field">
            <span>{i18n.t('dialog.name')}</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="dialog-host__input" type="text" bind:value bind:this={input} autofocus />
          </label>
        {:else}
          <p class="dialog-host__message">{i18n.text(current.message)}</p>
          {#if current.kind === 'info'}
            <ul class="dialog-host__items">
              {#each current.items as item, index (index)}
                <li>{i18n.text(item)}</li>
              {/each}
            </ul>
          {/if}
        {/if}
        <div class="dialog-host__actions">
          {#if current.kind !== 'info'}
            <Button onclick={cancel}>{i18n.t('dialog.cancel')}</Button>
          {/if}
          <Button type="submit" variant="primary">{i18n.t('dialog.ok')}</Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .dialog-host {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .dialog-host__dialog {
    width: min(420px, calc(100% - 2 * var(--space-l)));
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .dialog-host__form {
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .dialog-host__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .dialog-host__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    color: var(--color-muted);
  }

  .dialog-host__input {
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
  }

  .dialog-host__message {
    margin: 0;
    line-height: 1.5;
  }

  .dialog-host__items {
    max-height: 40vh;
    margin: 0;
    padding-left: var(--space-l);
    overflow-y: auto;
    line-height: 1.6;
  }

  .dialog-host__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
