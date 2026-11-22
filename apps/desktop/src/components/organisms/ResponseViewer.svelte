<script lang="ts">
  import { fitsCodeView, formatBytes, formatDuration } from '../../lib/format';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { RunOutcome } from '../../lib/types';
  import CodeViewer from '../molecules/CodeViewer.svelte';
  import TestResults from '../molecules/TestResults.svelte';

  interface Props {
    outcome: RunOutcome | null;
    sending: boolean;
  }

  let { outcome, sending }: Props = $props();

  type Tab = 'body' | 'headers' | 'tests' | 'logs';
  let tab = $state<Tab>('body');
  let logs = $derived(outcome?.logs ?? []);
  let tests = $derived(outcome?.tests ?? []);
  let passedTests = $derived(tests.filter((test) => test.passed).length);
  /** The test and log tabs only when scripts ran tests or logged. */
  let tabs = $derived<Tab[]>([
    'body',
    'headers',
    ...(tests.length > 0 ? (['tests'] as const) : []),
    ...(logs.length > 0 ? (['logs'] as const) : []),
  ]);
  let shown = $derived<Tab>(tabs.includes(tab) ? tab : 'body');
</script>

<section class="response" aria-label={i18n.t('response.title')} aria-busy={sending}>
  {#if outcome === null}
    <p class="response__empty">{i18n.t('response.empty')}</p>
  {:else if outcome.kind === 'failed'}
    <div class="response__failure" role="alert">
      <strong>{i18n.t('response.failed')}</strong>
      <span>{i18n.text(outcome.error)}</span>
    </div>
    {#if tests.length > 0}
      <TestResults {tests} />
    {/if}
    {#if logs.length > 0}
      <pre class="response__logs" aria-label={i18n.t('response.logs')}>{logs.join('\n')}</pre>
    {/if}
  {:else}
    <div class="response__meta">
      <span class="response__status response__status--{outcome.statusClass}">
        {outcome.status}
        {outcome.reason ?? ''}
      </span>
      <span class="response__figure">
        {i18n.t('response.time')} <strong>{formatDuration(outcome.elapsedMs)}</strong>
      </span>
      <span class="response__figure">
        {i18n.t('response.size')} <strong>{formatBytes(outcome.sizeBytes)}</strong>
      </span>
    </div>

    {#if outcome.scriptError}
      <p class="response__script-error" role="alert">{i18n.text(outcome.scriptError)}</p>
    {/if}

    <div class="response__tabs" role="tablist">
      {#each tabs as name (name)}
        <button
          type="button"
          role="tab"
          class="response__tab"
          class:response__tab--active={shown === name}
          aria-selected={shown === name}
          onclick={() => (tab = name)}
        >
          {i18n.t(`response.${name}`)}
          {#if name === 'headers'}<span class="response__count">{outcome.headers.length}</span>{/if}
          {#if name === 'logs'}<span class="response__count">{logs.length}</span>{/if}
          {#if name === 'tests'}
            <span
              class="response__count"
              class:response__count--failed={passedTests < tests.length}
              class:response__count--passed={passedTests === tests.length}
            >
              {passedTests}/{tests.length}
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="response__panel" role="tabpanel">
      {#if shown === 'logs'}
        <pre class="response__logs">{logs.join('\n')}</pre>
      {:else if shown === 'tests'}
        <TestResults {tests} />
      {:else if shown === 'body'}
        {#if outcome.body.binary}
          <p class="response__note">
            {i18n.t('response.body.binary', { size: formatBytes(outcome.sizeBytes) })}
          </p>
        {:else if outcome.body.text === ''}
          <p class="response__note">{i18n.t('response.body.none')}</p>
        {:else}
          {#if outcome.body.truncated}
            <p class="response__note">
              {i18n.t('response.body.truncated', { size: formatBytes(outcome.body.text.length) })}
            </p>
          {/if}
          {#if fitsCodeView(outcome.body.text)}
            <div class="response__body response__body--code">
              <CodeViewer
                text={outcome.body.text}
                isJson={outcome.body.pretty}
                label={i18n.t('response.body')}
              />
            </div>
          {:else}
            <pre class="response__body">{outcome.body.text}</pre>
          {/if}
        {/if}
      {:else}
        <table class="response__headers">
          <tbody>
            {#each outcome.headers as header, index (index)}
              <tr>
                <th scope="row">{header.name}</th>
                <td>{header.value}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {/if}
</section>

<style>
  .response {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .response__empty,
  .response__note {
    margin: 0;
    padding: var(--space-l);
    color: var(--color-muted);
  }

  .response__logs {
    margin: 0;
    padding: var(--space-m) var(--space-l);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .response__script-error {
    margin: 0;
    padding: var(--space-s) var(--space-l);
    background: var(--color-error-bg);
    color: var(--color-error);
    font-size: var(--font-size-s);
  }

  .response__failure {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-l);
    background: var(--color-error-bg);
    color: var(--color-error);
  }

  .response__meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-l);
    padding: var(--space-m) var(--space-l);
    border-bottom: 1px solid var(--color-border);
  }

  .response__status {
    font-weight: 700;
  }

  .response__status--success {
    color: var(--color-success);
  }

  .response__status--redirection,
  .response__status--informational {
    color: var(--color-redirect);
  }

  .response__status--clientError {
    color: var(--color-warning);
  }

  .response__status--serverError,
  .response__status--nonstandard {
    color: var(--color-error);
  }

  .response__figure {
    color: var(--color-muted);
  }

  .response__figure strong {
    color: var(--color-text);
    font-weight: 500;
  }

  .response__tabs {
    display: flex;
    gap: var(--space-xs);
    padding: 0 var(--space-m);
    border-bottom: 1px solid var(--color-border);
  }

  .response__tab {
    padding: var(--space-m) var(--space-s);
    border: 0;
    background: transparent;
    color: var(--color-muted);
    font-weight: 500;
    cursor: pointer;
  }

  .response__tab--active {
    color: var(--color-text);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .response__count {
    margin-left: var(--space-xs);
    padding: 0 6px;
    border-radius: 9px;
    background: var(--color-raised);
    font-size: var(--font-size-s);
  }

  .response__count--passed {
    color: var(--color-success);
  }

  .response__count--failed {
    color: var(--color-error);
  }

  .response__panel {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  /* The code viewer scrolls itself and fills the panel. */
  .response__panel:has(.response__body--code) {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .response__body--code {
    flex: 1;
    min-height: 0;
    padding: 0;
  }

  .response__body {
    margin: 0;
    padding: var(--space-l);
    background: var(--color-code-bg);
    font-family: var(--font-mono);
    font-size: var(--font-size-m);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .response__headers {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
  }

  .response__headers th,
  .response__headers td {
    padding: var(--space-s) var(--space-l);
    border-bottom: 1px solid var(--color-border);
    text-align: left;
    vertical-align: top;
  }

  .response__headers th {
    font-weight: 500;
    white-space: nowrap;
  }
</style>
