<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import RowsEditor, { type RowLabels } from '../molecules/RowsEditor.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { EnvironmentDraft, UserText } from '../../lib/types';

  interface Props {
    /** A copy of the environment; edited in place. */
    draft: EnvironmentDraft;
    /** Why saving or deleting failed. */
    error: UserText | null;
    onsave: () => void;
    oncancel: () => void;
    ondelete: () => void;
  }

  let { draft, error, onsave, oncancel, ondelete }: Props = $props();

  const VARIABLE_LABELS: RowLabels = {
    enabled: 'env.variable_enabled',
    name: 'env.variable_name',
    value: 'env.variable_value',
    add: 'env.add_variable',
    remove: 'env.remove_variable',
    empty: 'env.no_variables',
  };

  function submit(event: SubmitEvent) {
    event.preventDefault();
    onsave();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      oncancel();
    }
  }
</script>

<div class="environment-editor">
  <div
    class="environment-editor__dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="environment-editor-title"
    tabindex="-1"
    onkeydown={keydown}
  >
    <form class="environment-editor__form" onsubmit={submit}>
      <h2 id="environment-editor-title" class="environment-editor__title">
        {i18n.t('env.title')}
      </h2>
      <label class="environment-editor__field">
        <span>{i18n.t('dialog.name')}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="environment-editor__input"
          type="text"
          bind:value={draft.name}
          readonly={draft.readOnly}
          autofocus
        />
      </label>
      {#if draft.readOnly}
        <p class="environment-editor__note">{i18n.t('env.read_only')}</p>
      {/if}
      <div class="environment-editor__rows">
        <RowsEditor bind:rows={draft.variables} labels={VARIABLE_LABELS} />
      </div>
      {#if error}
        <p class="environment-editor__error" role="alert">{i18n.text(error)}</p>
      {/if}
      <div class="environment-editor__actions">
        <Button onclick={ondelete} disabled={draft.readOnly}>{i18n.t('env.delete')}</Button>
        <span class="environment-editor__spacer"></span>
        <Button onclick={oncancel}>{i18n.t('dialog.cancel')}</Button>
        <Button type="submit" variant="primary" disabled={draft.readOnly}>
          {i18n.t('editor.save')}
        </Button>
      </div>
    </form>
  </div>
</div>

<style>
  .environment-editor {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .environment-editor__dialog {
    width: min(640px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    display: flex;
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .environment-editor__form {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .environment-editor__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .environment-editor__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    color: var(--color-muted);
  }

  .environment-editor__input {
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
  }

  .environment-editor__rows {
    min-height: 0;
    overflow-y: auto;
  }

  .environment-editor__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .environment-editor__error {
    margin: 0;
    color: var(--color-error);
  }

  .environment-editor__actions {
    display: flex;
    gap: var(--space-s);
  }

  .environment-editor__spacer {
    flex: 1;
  }
</style>
