<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import RowsEditor, { type RowLabels } from '../molecules/RowsEditor.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { VariablesState } from '../../lib/workspace/variables.svelte';

  let { variables }: { variables: VariablesState } = $props();

  const LABELS: RowLabels = {
    enabled: 'env.variable_enabled',
    name: 'env.variable_name',
    value: 'env.variable_value',
    add: 'env.add_variable',
    remove: 'env.remove_variable',
    empty: 'env.no_variables',
  };

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void variables.save();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      variables.close();
    }
  }
</script>

{#if variables.editing}
  {@const editing = variables.editing}
  <div class="variables-dialog">
    <div
      class="variables-dialog__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="variables-dialog-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <form class="variables-dialog__form" onsubmit={submit}>
        <h2 id="variables-dialog-title" class="variables-dialog__title">
          {i18n.t('variables.title', { name: editing.name })}
        </h2>
        <p class="variables-dialog__note">
          {i18n.t(editing.isCollection ? 'variables.hint_collection' : 'variables.hint_folder')}
        </p>
        <div class="variables-dialog__rows">
          <RowsEditor bind:rows={editing.variables} labels={LABELS} />
        </div>
        {#if variables.error}
          <p class="variables-dialog__error" role="alert">{i18n.text(variables.error)}</p>
        {/if}
        <div class="variables-dialog__actions">
          <Button onclick={() => variables.close()}>{i18n.t('dialog.cancel')}</Button>
          <Button type="submit" variant="primary" disabled={variables.saving}>
            {variables.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .variables-dialog {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .variables-dialog__dialog {
    width: min(640px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    display: flex;
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .variables-dialog__form {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .variables-dialog__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .variables-dialog__note {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .variables-dialog__rows {
    min-height: 0;
    overflow-y: auto;
  }

  .variables-dialog__error {
    margin: 0;
    color: var(--color-error);
  }

  .variables-dialog__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
