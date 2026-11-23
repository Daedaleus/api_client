<script lang="ts" module>
  /** A row of name, value and on/off – a header or an environment variable. */
  export interface Row {
    name: string;
    value: string;
    enabled: boolean;
    /** Only variables: the value is never shown (see `RowLabels.secret`). */
    secret?: boolean;
  }

  /** Translation keys for the texts of one kind of row. */
  export interface RowLabels {
    enabled: string;
    name: string;
    value: string;
    add: string;
    remove: string;
    empty: string;
    /** Rows may be secret: a toggle per row, values masked. */
    secret?: {
      /** Label of the toggle. */
      toggle: string;
      /** Hint in the empty value field of a secret. */
      placeholder: string;
    };
  }
</script>

<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import Icon from '../atoms/Icon.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { rowsToText, textToRows } from '../../lib/rows-text';
  import type { VariableInfo } from '../../lib/variables';
  import VariableField from './VariableField.svelte';

  interface Props {
    rows: Row[];
    labels: RowLabels;
    /** Variables for suggestions in the value fields (requests, not environments). */
    variables?: ReadonlyMap<string, VariableInfo> | undefined;
    /**
     * Offer editing as text (`Name: value` per line) – for headers,
     * parameters and form fields, never for rows that may be secret.
     */
    textMode?: boolean;
  }

  let { rows = $bindable(), labels, variables, textMode = false }: Props = $props();

  /** Editing as text, and the text as typed (rows are read from it live). */
  let asText = $state(false);
  let text = $state('');

  /** Shown in the empty text field (protocol values, not translated). */
  const TEXT_EXAMPLE = 'Accept: application/json\n// X-Debug: 1';

  function toggleText() {
    if (!asText) text = rowsToText(rows);
    asText = !asText;
  }

  function typed(value: string) {
    text = value;
    rows = textToRows(value);
  }

  function add() {
    rows.push(
      labels.secret
        ? { name: '', value: '', enabled: true, secret: false }
        : { name: '', value: '', enabled: true },
    );
  }

  function remove(index: number) {
    rows.splice(index, 1);
  }
</script>

<div class="rows-editor">
  {#if textMode && !labels.secret}
    <div class="rows-editor__mode">
      <Button onclick={toggleText}>
        {i18n.t(asText ? 'rows.as_table' : 'rows.as_text')}
      </Button>
    </div>
  {/if}
  {#if asText}
    <textarea
      class="rows-editor__text"
      spellcheck="false"
      rows={Math.max(6, text.split('\n').length + 1)}
      aria-label={i18n.t('rows.text_label')}
      placeholder={TEXT_EXAMPLE}
      value={text}
      oninput={(event) => typed(event.currentTarget.value)}></textarea>
    <p class="rows-editor__hint">{i18n.t('rows.text_hint')}</p>
  {:else if rows.length > 0}
    <table class="rows-editor__table">
      <thead>
        <tr>
          <th class="rows-editor__toggle-cell">
            <span class="rows-editor__hidden">{i18n.t(labels.enabled)}</span>
          </th>
          <th>{i18n.t(labels.name)}</th>
          <th>{i18n.t(labels.value)}</th>
          {#if labels.secret}
            <th class="rows-editor__toggle-cell">
              <span class="rows-editor__hidden">{i18n.t(labels.secret.toggle)}</span>
            </th>
          {/if}
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
              {#if variables && !row.secret}
                <VariableField
                  class="rows-editor__input"
                  label={i18n.t(labels.value)}
                  {variables}
                  bind:value={row.value}
                />
              {:else}
                <input
                  class="rows-editor__input"
                  type={row.secret ? 'password' : 'text'}
                  spellcheck="false"
                  autocomplete="off"
                  aria-label={i18n.t(labels.value)}
                  placeholder={row.secret && labels.secret ? i18n.t(labels.secret.placeholder) : ''}
                  bind:value={row.value}
                />
              {/if}
            </td>
            {#if labels.secret}
              <td class="rows-editor__toggle-cell">
                <label class="rows-editor__secret" title={i18n.t(labels.secret.toggle)}>
                  <input
                    type="checkbox"
                    class="rows-editor__secret-input"
                    aria-label={i18n.t(labels.secret.toggle)}
                    bind:checked={row.secret}
                  />
                  <Icon name="lock" />
                </label>
              </td>
            {/if}
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
  {#if !asText}
    <div>
      <Button onclick={add}>{i18n.t(labels.add)}</Button>
    </div>
  {/if}
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

  /* Also the inputs inside VariableField (class passed down). */
  .rows-editor :global(.rows-editor__input) {
    width: 100%;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    font-family: var(--font-mono);
  }

  .rows-editor__row--disabled :global(.rows-editor__input) {
    color: var(--color-muted);
    text-decoration: line-through;
  }

  .rows-editor__secret {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    color: var(--color-muted);
    cursor: pointer;
  }

  .rows-editor__secret:has(:checked) {
    background: var(--color-raised);
    color: var(--color-accent);
  }

  .rows-editor__secret:has(:focus-visible) {
    outline: 2px solid var(--color-accent);
  }

  .rows-editor__secret-input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .rows-editor__empty {
    margin: 0;
    color: var(--color-muted);
  }

  .rows-editor__mode {
    display: flex;
    justify-content: flex-end;
  }

  .rows-editor__text {
    width: 100%;
    padding: var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
    resize: vertical;
  }

  .rows-editor__hint {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .rows-editor__hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
