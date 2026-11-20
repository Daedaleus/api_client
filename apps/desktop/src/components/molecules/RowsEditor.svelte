<script lang="ts" module>
  /** A row of name, value and on/off – a header or an environment variable. */
  export interface Row {
    name: string;
    value: string;
    enabled: boolean;
  }

  /** Translation keys for the texts of one kind of row. */
  export interface RowLabels {
    enabled: string;
    name: string;
    value: string;
    add: string;
    remove: string;
    empty: string;
  }
</script>

<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';

  let { rows = $bindable(), labels }: { rows: Row[]; labels: RowLabels } = $props();

  function add() {
    rows.push({ name: '', value: '', enabled: true });
  }

  function remove(index: number) {
    rows.splice(index, 1);
  }
</script>

<div class="rows-editor">
  {#if rows.length > 0}
    <table class="rows-editor__table">
      <thead>
        <tr>
          <th class="rows-editor__toggle-cell">
            <span class="rows-editor__hidden">{i18n.t(labels.enabled)}</span>
          </th>
          <th>{i18n.t(labels.name)}</th>
          <th>{i18n.t(labels.value)}</th>
          <th><span class="rows-editor__hidden">{i18n.t(labels.remove)}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row, index (index)}
          <tr class:rows-editor__row--disabled={!row.enabled}>
            <td class="rows-editor__toggle-cell">
              <input
                type="checkbox"
                aria-label={i18n.t(labels.enabled)}
                bind:checked={row.enabled}
              />
            </td>
            <td>
              <input
                class="rows-editor__input"
                type="text"
                spellcheck="false"
                aria-label={i18n.t(labels.name)}
                bind:value={row.name}
              />
            </td>
            <td>
              <input
                class="rows-editor__input"
                type="text"
                spellcheck="false"
                aria-label={i18n.t(labels.value)}
                bind:value={row.value}
              />
            </td>
            <td>
              <IconButton
                icon="trash"
                label={i18n.t(labels.remove)}
                onclick={() => remove(index)}
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="rows-editor__empty">{i18n.t(labels.empty)}</p>
  {/if}
  <div>
    <Button onclick={add}>{i18n.t(labels.add)}</Button>
  </div>
</div>

<style>
  .rows-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .rows-editor__table {
    width: 100%;
    border-collapse: collapse;
  }

  .rows-editor__table th {
    padding: var(--space-xs) var(--space-s);
    text-align: left;
    font-weight: 500;
    font-size: var(--font-size-s);
    color: var(--color-muted);
  }

  .rows-editor__table td {
    padding: 2px var(--space-xs);
  }

  .rows-editor__toggle-cell {
    width: 32px;
    text-align: center;
  }

  .rows-editor__input {
    width: 100%;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    font-family: var(--font-mono);
  }

  .rows-editor__row--disabled .rows-editor__input {
    color: var(--color-muted);
    text-decoration: line-through;
  }

  .rows-editor__empty {
    margin: 0;
    color: var(--color-muted);
  }

  .rows-editor__hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
