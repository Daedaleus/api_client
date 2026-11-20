<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { HeaderDraft } from '../../lib/types';

  let { headers = $bindable() }: { headers: HeaderDraft[] } = $props();

  function add() {
    headers.push({ name: '', value: '', enabled: true });
  }

  function remove(index: number) {
    headers.splice(index, 1);
  }
</script>

<div class="headers-editor">
  {#if headers.length > 0}
    <table class="headers-editor__table">
      <thead>
        <tr>
          <th class="headers-editor__toggle-cell">
            <span class="headers-editor__hidden">{i18n.t('editor.header_enabled')}</span>
          </th>
          <th>{i18n.t('editor.header_name')}</th>
          <th>{i18n.t('editor.header_value')}</th>
          <th><span class="headers-editor__hidden">{i18n.t('editor.remove_header')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each headers as header, index (index)}
          <tr class:headers-editor__row--disabled={!header.enabled}>
            <td class="headers-editor__toggle-cell">
              <input
                type="checkbox"
                aria-label={i18n.t('editor.header_enabled')}
                bind:checked={header.enabled}
              />
            </td>
            <td>
              <input
                class="headers-editor__input"
                type="text"
                spellcheck="false"
                aria-label={i18n.t('editor.header_name')}
                bind:value={header.name}
              />
            </td>
            <td>
              <input
                class="headers-editor__input"
                type="text"
                spellcheck="false"
                aria-label={i18n.t('editor.header_value')}
                bind:value={header.value}
              />
            </td>
            <td>
              <IconButton
                icon="trash"
                label={i18n.t('editor.remove_header')}
                onclick={() => remove(index)}
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="headers-editor__empty">{i18n.t('editor.no_headers')}</p>
  {/if}
  <div>
    <Button onclick={add}>{i18n.t('editor.add_header')}</Button>
  </div>
</div>

<style>
  .headers-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .headers-editor__table {
    width: 100%;
    border-collapse: collapse;
  }

  .headers-editor__table th {
    padding: var(--space-xs) var(--space-s);
    text-align: left;
    font-weight: 500;
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }

  .headers-editor__table td {
    padding: 2px var(--space-xs);
  }

  .headers-editor__toggle-cell {
    width: 32px;
    text-align: center;
  }

  .headers-editor__input {
    width: 100%;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    font-family: var(--font-mono);
  }

  .headers-editor__row--disabled .headers-editor__input {
    color: var(--color-muted);
    text-decoration: line-through;
  }

  .headers-editor__empty {
    margin: 0;
    color: var(--color-muted);
  }

  .headers-editor__hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
