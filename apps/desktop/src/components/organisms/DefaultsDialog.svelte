<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import AuthEditor from '../molecules/AuthEditor.svelte';
  import CertificateCheckField from '../molecules/CertificateCheckField.svelte';
  import DocsEditor from '../molecules/DocsEditor.svelte';
  import RowsEditor, { type RowLabels } from '../molecules/RowsEditor.svelte';
  import ScriptsEditor from '../molecules/ScriptsEditor.svelte';
  import TransportFields from '../molecules/TransportFields.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { ScriptAssist } from '../../lib/script-assist';
  import type { DefaultsState } from '../../lib/workspace/defaults.svelte';

  interface Props {
    defaults: DefaultsState;
    /** Checker and completion for the scripts. */
    assist?: ScriptAssist | undefined;
  }

  let { defaults, assist }: Props = $props();

  const LABELS: RowLabels = {
    enabled: 'env.variable_enabled',
    name: 'env.variable_name',
    value: 'env.variable_value',
    add: 'env.add_variable',
    remove: 'env.remove_variable',
    empty: 'env.no_variables',
  };

  type Section = 'variables' | 'auth' | 'scripts' | 'settings' | 'docs';
  const SECTIONS: Section[] = ['variables', 'auth', 'scripts', 'settings', 'docs'];
  let section = $state<Section>('variables');

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void defaults.save();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      defaults.close();
    }
  }
</script>

{#if defaults.editing}
  {@const editing = defaults.editing}
  <div class="defaults-dialog">
    <div
      class="defaults-dialog__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="defaults-dialog-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <form class="defaults-dialog__form" onsubmit={submit}>
        <h2 id="defaults-dialog-title" class="defaults-dialog__title">
          {i18n.t('defaults.title', { name: editing.name })}
        </h2>
        <p class="defaults-dialog__note">
          {i18n.t(editing.isCollection ? 'variables.hint_collection' : 'variables.hint_folder')}
        </p>
        <div class="defaults-dialog__sections" role="tablist">
          {#each SECTIONS as name (name)}
            <button
              type="button"
              role="tab"
              class="defaults-dialog__section"
              class:defaults-dialog__section--active={section === name}
              aria-selected={section === name}
              onclick={() => (section = name)}
            >
              {i18n.t(`defaults.${name}`)}
            </button>
          {/each}
        </div>
        <div class="defaults-dialog__rows" role="tabpanel">
          {#if section === 'variables'}
            <RowsEditor bind:rows={editing.variables} labels={LABELS} />
          {:else if section === 'auth'}
            <AuthEditor bind:auth={editing.auth} root={editing.isCollection} />
          {:else if section === 'scripts'}
            <p class="defaults-dialog__note">{i18n.t('scripts.hint_around')}</p>
            <ScriptsEditor bind:scripts={editing.scripts} {assist} />
          {:else if section === 'docs'}
            <DocsEditor bind:docs={editing.docs} />
          {:else}
            <CertificateCheckField
              bind:check={editing.certificateCheck}
              root={editing.isCollection}
            />
            <TransportFields bind:transport={editing.transport} root={editing.isCollection} />
          {/if}
        </div>
        {#if defaults.error}
          <p class="defaults-dialog__error" role="alert">{i18n.text(defaults.error)}</p>
        {/if}
        <div class="defaults-dialog__actions">
          <Button onclick={() => defaults.close()}>{i18n.t('dialog.cancel')}</Button>
          <Button type="submit" variant="primary" disabled={defaults.saving}>
            {defaults.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .defaults-dialog {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .defaults-dialog__dialog {
    width: min(640px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    display: flex;
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .defaults-dialog__form {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .defaults-dialog__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .defaults-dialog__note {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .defaults-dialog__sections {
    display: flex;
    gap: var(--space-xs);
    border-bottom: 1px solid var(--color-border);
  }

  .defaults-dialog__section {
    padding: var(--space-s);
    border: 0;
    background: transparent;
    color: var(--color-muted);
    font-weight: 500;
    cursor: pointer;
  }

  .defaults-dialog__section--active {
    color: var(--color-text);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .defaults-dialog__rows {
    min-height: 0;
    overflow-y: auto;
  }

  .defaults-dialog__error {
    margin: 0;
    color: var(--color-error);
  }

  .defaults-dialog__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }
</style>
