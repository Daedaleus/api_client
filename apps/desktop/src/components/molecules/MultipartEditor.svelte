<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { MultipartFieldDraft } from '../../lib/types';

  interface Props {
    fields: MultipartFieldDraft[];
    /** Lets the user pick a file; resolves to its path or `null`. */
    onpickfile?: (() => Promise<string | null>) | undefined;
  }

  let { fields = $bindable(), onpickfile }: Props = $props();

  function add() {
    fields.push({ name: '', value: '', enabled: true, file: false });
  }

  async function pick(field: MultipartFieldDraft) {
    const path = await onpickfile?.();
    if (path) field.value = path;
  }
</script>

<div class="multipart-editor">
  {#if fields.length > 0}
    <table class="multipart-editor__table">
      <thead>
        <tr>
          <th class="multipart-editor__narrow">
            <span class="multipart-editor__hidden">{i18n.t('editor.field_enabled')}</span>
          </th>
          <th>{i18n.t('editor.field_name')}</th>
          <th class="multipart-editor__narrow">{i18n.t('editor.field_file')}</th>
          <th>{i18n.t('editor.field_value')}</th>
          <th><span class="multipart-editor__hidden">{i18n.t('editor.remove_field')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each fields as field, index (index)}
          <tr class:multipart-editor__row--disabled={!field.enabled}>
            <td class="multipart-editor__narrow">
              <input
                type="checkbox"
                aria-label={i18n.t('editor.field_enabled')}
                bind:checked={field.enabled}
              />
            </td>
            <td>
              <input
                class="multipart-editor__input"
                type="text"
                spellcheck="false"
                aria-label={i18n.t('editor.field_name')}
                bind:value={field.name}
              />
            </td>
            <td class="multipart-editor__narrow">
              <input
                type="checkbox"
                aria-label={i18n.t('editor.field_file')}
                bind:checked={field.file}
              />
            </td>
            <td>
              <span class="multipart-editor__value">
                <input
                  class="multipart-editor__input"
                  type="text"
                  spellcheck="false"
                  aria-label={i18n.t('editor.field_value')}
                  placeholder={field.file ? i18n.t('editor.file_placeholder') : ''}
                  bind:value={field.value}
                />
                {#if field.file && onpickfile}
                  <Button onclick={() => pick(field)}>{i18n.t('editor.pick_file')}</Button>
                {/if}
              </span>
            </td>
            <td>
              <IconButton
                icon="trash"
                label={i18n.t('editor.remove_field')}
                onclick={() => fields.splice(index, 1)}
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="multipart-editor__empty">{i18n.t('editor.no_fields')}</p>
  {/if}
  <div>
    <Button onclick={add}>{i18n.t('editor.add_field')}</Button>
  </div>
</div>

<style>
  .multipart-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .multipart-editor__table {
    width: 100%;
    border-collapse: collapse;
  }

  .multipart-editor__table th {
    padding: var(--space-xs) var(--space-s);
    text-align: left;
    font-weight: 500;
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }

  .multipart-editor__table td {
    padding: 2px var(--space-xs);
  }

  .multipart-editor__narrow {
    width: 48px;
    text-align: center;
  }

  .multipart-editor__value {
    display: flex;
    gap: var(--space-xs);
  }

  .multipart-editor__input {
    width: 100%;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    font-family: var(--font-mono);
  }

  .multipart-editor__row--disabled .multipart-editor__input {
    color: var(--color-muted);
    text-decoration: line-through;
  }

  .multipart-editor__hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .multipart-editor__empty {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
