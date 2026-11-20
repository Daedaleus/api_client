<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { RecipientsState } from '../../lib/workspace/recipients.svelte';

  let { recipients }: { recipients: RecipientsState } = $props();

  let newKey = $state('');
  let copied = $state(false);

  function add() {
    if (recipients.add(newKey)) newKey = '';
  }

  function addOnEnter(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      add();
    }
  }

  async function copyOwn() {
    const own = recipients.editing?.own;
    if (!own) return;
    try {
      await navigator.clipboard.writeText(own);
      copied = true;
    } catch {
      // No clipboard access: the key is still selectable in the field.
    }
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void recipients.save();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      recipients.close();
    }
  }
</script>

{#if recipients.editing}
  {@const editing = recipients.editing}
  <div class="recipients-dialog">
    <div
      class="recipients-dialog__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="recipients-dialog-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <form class="recipients-dialog__form" onsubmit={submit}>
        <h2 id="recipients-dialog-title" class="recipients-dialog__title">
          {i18n.t('recipients.title')}
        </h2>
        <p class="recipients-dialog__note">{i18n.t('recipients.intro')}</p>

        <label class="recipients-dialog__field">
          <span>{i18n.t('recipients.own')}</span>
          <span class="recipients-dialog__row">
            <input
              class="recipients-dialog__input recipients-dialog__input--own"
              type="text"
              readonly
              value={editing.own}
              onfocus={(event) => event.currentTarget.select()}
            />
            <Button onclick={copyOwn}>
              {copied ? i18n.t('recipients.copied') : i18n.t('recipients.copy')}
            </Button>
          </span>
        </label>

        <div class="recipients-dialog__field">
          <span>{i18n.t('recipients.list')}</span>
          <ul class="recipients-dialog__list">
            {#each editing.list as key, index (index)}
              <li class="recipients-dialog__item">
                <code class="recipients-dialog__key">{key}</code>
                {#if key === editing.own}
                  <span class="recipients-dialog__you">{i18n.t('recipients.you')}</span>
                {/if}
                <IconButton
                  icon="trash"
                  label={i18n.t('recipients.remove')}
                  onclick={() => recipients.remove(index)}
                />
              </li>
            {/each}
          </ul>
          {#if recipients.withoutOwnKey}
            <p class="recipients-dialog__warning" role="status">
              {i18n.t('recipients.without_you')}
            </p>
          {/if}
        </div>

        <label class="recipients-dialog__field">
          <span>{i18n.t('recipients.new')}</span>
          <span class="recipients-dialog__row">
            <input
              class="recipients-dialog__input"
              type="text"
              spellcheck="false"
              autocomplete="off"
              bind:value={newKey}
              onkeydown={addOnEnter}
            />
            <Button onclick={add} disabled={newKey.trim() === ''}>
              {i18n.t('recipients.add')}
            </Button>
          </span>
        </label>

        {#if recipients.error}
          <p class="recipients-dialog__error" role="alert">{i18n.text(recipients.error)}</p>
        {/if}
        <div class="recipients-dialog__actions">
          <Button onclick={() => recipients.close()}>{i18n.t('dialog.cancel')}</Button>
          <Button type="submit" variant="primary" disabled={recipients.saving}>
            {recipients.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .recipients-dialog {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .recipients-dialog__dialog {
    width: min(640px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    display: flex;
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .recipients-dialog__form {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
    overflow-y: auto;
  }

  .recipients-dialog__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .recipients-dialog__note {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .recipients-dialog__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    color: var(--color-muted);
  }

  .recipients-dialog__row {
    display: flex;
    gap: var(--space-s);
  }

  .recipients-dialog__input {
    flex: 1;
    min-width: 0;
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-mono);
  }

  .recipients-dialog__input--own {
    background: var(--color-raised);
  }

  .recipients-dialog__list {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .recipients-dialog__item {
    display: flex;
    align-items: center;
    gap: var(--space-s);
    padding: var(--space-xs) var(--space-s);
    color: var(--color-text);
  }

  .recipients-dialog__item + .recipients-dialog__item {
    border-top: 1px solid var(--color-border);
  }

  .recipients-dialog__key {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
  }

  .recipients-dialog__you {
    padding: 0 6px;
    border-radius: 9px;
    background: var(--color-raised);
    font-size: var(--font-size-s);
  }

  .recipients-dialog__warning {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }

  .recipients-dialog__error {
    margin: 0;
    color: var(--color-error);
  }

  .recipients-dialog__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
