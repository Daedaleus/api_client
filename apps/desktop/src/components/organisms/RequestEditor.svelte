<script lang="ts">
  import type { Snippet } from 'svelte';
  import Button from '../atoms/Button.svelte';
  import AuthEditor from '../molecules/AuthEditor.svelte';
  import CertificateCheckField from '../molecules/CertificateCheckField.svelte';
  import BodyEditor from '../molecules/BodyEditor.svelte';
  import RowsEditor, { type RowLabels } from '../molecules/RowsEditor.svelte';
  import ParamsEditor from '../molecules/ParamsEditor.svelte';
  import ScriptsEditor from '../molecules/ScriptsEditor.svelte';
  import { fillPathParams, pathParamNames, queryParams } from '../../lib/params';
  import UrlBar from '../molecules/UrlBar.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { ScriptAssist } from '../../lib/script-assist';
  import { hasPlaceholders, preview, type VariableInfo } from '../../lib/variables';
  import type { RequestTab } from '../../lib/workspace/request-tab.svelte';

  interface Props {
    tab: RequestTab;
    onsend: () => void;
    onsave: () => void;
    /** The values `{{variables}}` get (all places, the stronger winning). */
    variables?: ReadonlyMap<string, string>;
    /** The same with where each comes from, for suggestions in the fields. */
    variableInfo?: ReadonlyMap<string, VariableInfo>;
    /** The environment picker, shown next to the name of a saved request. */
    environment?: Snippet;
    /** Whether the server certificate is checked (all levels considered). */
    checksCertificates?: boolean;
    /** The request as a cURL command (`null` if it cannot be built). */
    oncopycurl?: (() => Promise<string | null>) | undefined;
    /** Lets the user pick a file for the body. */
    onpickfile?: () => Promise<string | null>;
    /** Checker and completion for the scripts. */
    assist?: ScriptAssist | undefined;
  }

  let {
    tab,
    onsend,
    onsave,
    variables = new Map(),
    variableInfo = new Map(),
    environment,
    onpickfile,
    checksCertificates = true,
    oncopycurl,
    assist,
  }: Props = $props();

  let copied = $state(false);

  async function copyCurl() {
    const command = await oncopycurl?.();
    if (command === null || command === undefined) return;
    try {
      await navigator.clipboard.writeText(command);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // No clipboard access: nothing to report beyond the unchanged button.
    }
  }

  // What the URL looks like with its `{{variables}}` filled in.
  let urlPreview = $derived.by(() => {
    const url = fillPathParams(tab.draft.url, tab.draft.pathParams);
    return hasPlaceholders(url) ? preview(url, variables) : null;
  });

  const HEADER_LABELS: RowLabels = {
    enabled: 'editor.header_enabled',
    name: 'editor.header_name',
    value: 'editor.header_value',
    add: 'editor.add_header',
    remove: 'editor.remove_header',
    empty: 'editor.no_headers',
  };

  const VARIABLE_LABELS: RowLabels = {
    enabled: 'env.variable_enabled',
    name: 'env.variable_name',
    value: 'env.variable_value',
    add: 'env.add_variable',
    remove: 'env.remove_variable',
    empty: 'editor.no_variables',
  };

  type Section = 'params' | 'headers' | 'body' | 'auth' | 'variables' | 'settings' | 'scripts';
  const SECTIONS: Section[] = [
    'params',
    'headers',
    'body',
    'auth',
    'variables',
    'settings',
    'scripts',
  ];
  let section = $state<Section>('params');
  let activeParams = $derived(
    queryParams(tab.draft.url).length + pathParamNames(tab.draft.url).length,
  );
  let scriptCount = $derived(
    [tab.draft.scripts.preRequest, tab.draft.scripts.postResponse].filter(
      (script) => script.trim() !== '',
    ).length,
  );
  let activeVariables = $derived(
    tab.draft.variables.filter((variable) => variable.enabled && variable.name.trim() !== '')
      .length,
  );
  let activeHeaders = $derived(
    tab.draft.headers.filter((header) => header.enabled && header.name.trim() !== '').length,
  );
</script>

<div class="request-editor">
  {#if tab.saved}
    <div class="request-editor__header">
      <label class="request-editor__name">
        <span class="request-editor__label">{i18n.t('editor.name')}</span>
        <input class="request-editor__input" type="text" bind:value={tab.draft.name} />
      </label>
      {@render environment?.()}
      <Button
        variant={tab.dirty ? 'primary' : 'secondary'}
        disabled={tab.saving || tab.draft.readOnly}
        onclick={onsave}
      >
        {tab.saving ? i18n.t('editor.saving') : i18n.t('editor.save')}
      </Button>
    </div>
    {#if tab.draft.readOnly}
      <p class="request-editor__note">{i18n.t('editor.read_only')}</p>
    {/if}
  {:else}
    <p class="request-editor__note">{i18n.t('editor.scratch')}</p>
  {/if}
  <UrlBar
    bind:method={tab.draft.method}
    bind:url={tab.draft.url}
    sending={tab.sending}
    error={tab.error}
    {onsend}
    ondismisserror={() => (tab.error = null)}
    variables={variableInfo}
  />
  {#if !checksCertificates}
    <p class="request-editor__insecure" role="status">{i18n.t('tls.warning')}</p>
  {/if}
  {#if urlPreview}
    <p class="request-editor__preview">
      {i18n.t('editor.preview', { url: urlPreview.text })}
    </p>
    {#if urlPreview.missing.length > 0}
      <p class="request-editor__warning">
        {i18n.t('editor.variables_missing', { names: urlPreview.missing.join(', ') })}
      </p>
    {/if}
  {/if}
  <div class="request-editor__sections" role="tablist" aria-label={i18n.t('editor.sections')}>
    {#each SECTIONS as name (name)}
      <button
        type="button"
        role="tab"
        class="request-editor__section"
        class:request-editor__section--active={section === name}
        aria-selected={section === name}
        onclick={() => (section = name)}
      >
        {i18n.t(`editor.${name}`)}
        {#if name === 'params' && activeParams > 0}
          <span class="request-editor__count">{activeParams}</span>
        {:else if name === 'headers' && activeHeaders > 0}
          <span class="request-editor__count">{activeHeaders}</span>
        {:else if name === 'body' && tab.draft.body.kind !== 'none'}
          <span class="request-editor__count">{i18n.t(`editor.body_${tab.draft.body.kind}`)}</span>
        {:else if name === 'auth' && tab.draft.auth.kind !== 'inherit'}
          <span class="request-editor__count">{i18n.t(`auth.badge_${tab.draft.auth.kind}`)}</span>
        {:else if name === 'variables' && activeVariables > 0}
          <span class="request-editor__count">{activeVariables}</span>
        {:else if name === 'scripts' && scriptCount > 0}
          <span class="request-editor__count">{scriptCount}</span>
        {/if}
      </button>
    {/each}
    {#if oncopycurl}
      <span class="request-editor__tools">
        <Button onclick={copyCurl}>{copied ? i18n.t('curl.copied') : i18n.t('curl.copy')}</Button>
      </span>
    {/if}
  </div>
  {#if tab.notice}
    <p class="request-editor__note">{i18n.text(tab.notice)}</p>
  {/if}
  <div class="request-editor__panel" role="tabpanel">
    {#if section === 'params'}
      <ParamsEditor
        bind:url={tab.draft.url}
        bind:disabledQuery={tab.draft.disabledQuery}
        bind:pathParams={tab.draft.pathParams}
        variables={variableInfo}
      />
    {:else if section === 'headers'}
      <RowsEditor bind:rows={tab.draft.headers} labels={HEADER_LABELS} variables={variableInfo} />
    {:else if section === 'body'}
      <BodyEditor bind:body={tab.draft.body} {onpickfile} />
    {:else if section === 'auth'}
      <AuthEditor bind:auth={tab.draft.auth} />
    {:else if section === 'settings'}
      <CertificateCheckField bind:check={tab.draft.certificateCheck} />
    {:else if section === 'scripts'}
      <ScriptsEditor bind:scripts={tab.draft.scripts} {assist} />
    {:else}
      <p class="request-editor__note">{i18n.t('editor.variables_hint')}</p>
      <RowsEditor bind:rows={tab.draft.variables} labels={VARIABLE_LABELS} />
    {/if}
  </div>
</div>

<style>
  .request-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .request-editor__header {
    display: flex;
    gap: var(--space-s);
    align-items: center;
  }

  .request-editor__name {
    flex: 1;
    min-width: 0;
  }

  .request-editor__label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .request-editor__input {
    width: 100%;
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .request-editor__input:hover,
  .request-editor__input:focus {
    border-color: var(--color-border);
    background: var(--color-surface);
  }

  .request-editor__sections {
    display: flex;
    gap: var(--space-xs);
    border-bottom: 1px solid var(--color-border);
  }

  .request-editor__section {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-s) var(--space-s);
    border: 0;
    background: transparent;
    color: var(--color-muted);
    font-weight: 500;
    cursor: pointer;
  }

  .request-editor__section--active {
    color: var(--color-text);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .request-editor__tools {
    margin-left: auto;
    padding-bottom: var(--space-xs);
  }

  .request-editor__count {
    padding: 0 6px;
    border-radius: 9px;
    background: var(--color-raised);
    font-size: var(--font-size-s);
  }

  .request-editor__panel {
    max-height: 40vh;
    overflow-y: auto;
  }

  .request-editor__preview {
    margin: 0;
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    overflow-wrap: anywhere;
  }

  .request-editor__insecure {
    margin: 0;
    padding: var(--space-xs) var(--space-s);
    border-left: 3px solid var(--color-warning);
    border-radius: 4px;
    background: var(--color-raised);
    color: var(--color-warning);
    font-size: var(--font-size-s);
    font-weight: 600;
  }

  .request-editor__warning {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }

  .request-editor__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
