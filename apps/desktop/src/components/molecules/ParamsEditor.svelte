<script lang="ts">
  import RowsEditor, { type Row, type RowLabels } from './RowsEditor.svelte';
  import VariableField from './VariableField.svelte';
  import type { VariableInfo } from '../../lib/variables';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import { pathParamNames, queryParams, withQueryParams, type Param } from '../../lib/params';

  interface Props {
    /** The URL; switched-on query parameters live in it. */
    url: string;
    /** Query parameters switched off (kept, not sent). */
    disabledQuery: Param[];
    /** Values of the path variables (`:name`) in the URL. */
    pathParams: Param[];
    /** Variables for suggestions in the value fields. */
    variables?: ReadonlyMap<string, VariableInfo>;
  }

  let {
    url = $bindable(),
    disabledQuery = $bindable(),
    pathParams = $bindable(),
    variables = new Map(),
  }: Props = $props();

  const QUERY_LABELS: RowLabels = {
    enabled: 'params.enabled',
    name: 'params.name',
    value: 'params.value',
    add: 'params.add',
    remove: 'params.remove',
    empty: 'params.none',
  };

  /** The table rows for the URL and the switched-off parameters. */
  function rowsFor(url: string, disabled: Param[]): Row[] {
    return [
      ...queryParams(url).map((param) => ({ ...param, enabled: true })),
      ...disabled.map((param) => ({ ...param, enabled: false })),
    ];
  }

  let rows = $state<Row[]>([]);
  /** The rows as last built from the URL – edits are only written back if they differ. */
  let built = '';

  // URL typed or changed elsewhere: rebuild the table.
  $effect(() => {
    const next = rowsFor(url, disabledQuery);
    const text = JSON.stringify(next);
    if (text !== built) {
      built = text;
      rows = next;
    }
  });

  // Table edited: write the URL and the switched-off parameters back –
  // only what changed, so the table is not rebuilt (and a new, still empty
  // row kept).
  $effect(() => {
    const current = $state.snapshot(rows);
    const text = JSON.stringify(current);
    if (text === built) return;
    built = text;
    const nextUrl = withQueryParams(
      url,
      current.filter((row) => row.enabled),
    );
    const nextDisabled = current
      .filter((row) => !row.enabled && row.name !== '')
      .map(({ name, value }) => ({ name, value }));
    if (nextUrl !== url) url = nextUrl;
    if (JSON.stringify(nextDisabled) !== JSON.stringify(disabledQuery)) {
      disabledQuery = nextDisabled;
    }
  });

  let pathNames = $derived(pathParamNames(url));

  function pathValue(name: string): string {
    return pathParams.find((param) => param.name === name)?.value ?? '';
  }

  function setPathValue(name: string, value: string) {
    // Only the variables in the URL are kept.
    pathParams = pathNames.map((known) => ({
      name: known,
      value: known === name ? value : pathValue(known),
    }));
  }
</script>

<div class="params-editor">
  <section class="params-editor__group" aria-label={i18n.t('params.query')}>
    <h3 class="params-editor__title">{i18n.t('params.query')}</h3>
    <RowsEditor bind:rows labels={QUERY_LABELS} {variables} textMode />
  </section>

  <section class="params-editor__group" aria-label={i18n.t('params.path')}>
    <h3 class="params-editor__title">{i18n.t('params.path')}</h3>
    {#if pathNames.length === 0}
      <p class="params-editor__hint">{i18n.t('params.path_hint')}</p>
    {:else}
      <table class="params-editor__table">
        <tbody>
          {#each pathNames as name (name)}
            <tr>
              <th class="params-editor__name" scope="row">:{name}</th>
              <td>
                <VariableField
                  class="params-editor__input"
                  label={i18n.t('params.path_value', { name })}
                  {variables}
                  bind:value={() => pathValue(name), (value) => setPathValue(name, value)}
                />
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

<style>
  .params-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-l);
  }

  .params-editor__group {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .params-editor__title {
    margin: 0;
    font-size: var(--font-size-s);
    font-weight: 600;
    color: var(--color-muted);
  }

  .params-editor__hint {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .params-editor__table {
    width: 100%;
    border-collapse: collapse;
  }

  .params-editor__name {
    width: 30%;
    padding: var(--space-xs) var(--space-s) var(--space-xs) 0;
    font-family: var(--font-mono);
    font-weight: 400;
    text-align: left;
  }

  /* The input lives in VariableField; the class is passed down. */
  .params-editor :global(.params-editor__input) {
    width: 100%;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
  }
</style>
