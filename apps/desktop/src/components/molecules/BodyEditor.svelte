<script lang="ts">
  import { BODY_KINDS, type BodyKind, bodyContent, jsonProblem, withKind } from '../../lib/body';
  import Button from '../atoms/Button.svelte';
  import MultipartEditor from './MultipartEditor.svelte';
  import RowsEditor, { type RowLabels } from './RowsEditor.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { BodyDraft } from '../../lib/types';

  interface Props {
    body: BodyDraft;
    /** Lets the user pick a file; resolves to its path or `null`. */
    onpickfile?: (() => Promise<string | null>) | undefined;
  }

  let { body = $bindable(), onpickfile }: Props = $props();

  const FIELD_LABELS: RowLabels = {
    enabled: 'editor.field_enabled',
    name: 'editor.field_name',
    value: 'editor.field_value',
    add: 'editor.add_field',
    remove: 'editor.remove_field',
    empty: 'editor.no_fields',
  };

  async function pickBinary() {
    const path = await onpickfile?.();
    if (path && body.kind === 'binary') body.file = path;
  }

  let problem = $derived(body.kind === 'json' ? jsonProblem(body.content) : null);

  function choose(kind: BodyKind) {
    body = withKind(body, kind);
  }

  function edit(event: Event) {
    const content = (event.currentTarget as HTMLTextAreaElement).value;
    if (body.kind === 'json' || body.kind === 'text') body = { kind: body.kind, content };
  }
</script>

<div class="body-editor">
  <div class="body-editor__kinds" role="radiogroup" aria-label={i18n.t('editor.body_kind')}>
    {#each BODY_KINDS as kind (kind)}
      <label class="body-editor__kind" class:body-editor__kind--active={body.kind === kind}>
        <input
          type="radio"
          name="body-kind"
          value={kind}
          checked={body.kind === kind}
          onchange={() => choose(kind)}
        />
        {i18n.t(`editor.body_${kind}`)}
      </label>
    {/each}
  </div>

  {#if body.kind === 'none'}
    <p class="body-editor__note">{i18n.t('editor.body_none_note')}</p>
  {:else if body.kind === 'form'}
    <RowsEditor bind:rows={body.fields} labels={FIELD_LABELS} textMode />
  {:else if body.kind === 'multipart'}
    <MultipartEditor bind:fields={body.fields} {onpickfile} />
  {:else if body.kind === 'binary'}
    <div class="body-editor__file">
      <label class="body-editor__file-field">
        <span class="body-editor__file-label">{i18n.t('editor.body_file')}</span>
        <input
          class="body-editor__file-input"
          type="text"
          spellcheck="false"
          placeholder={i18n.t('editor.file_placeholder')}
          bind:value={body.file}
        />
      </label>
      {#if onpickfile}
        <Button onclick={pickBinary}>{i18n.t('editor.pick_file')}</Button>
      {/if}
    </div>
    <p class="body-editor__note">{i18n.t('editor.file_note')}</p>
  {:else}
    <textarea
      class="body-editor__text"
      spellcheck="false"
      aria-label={i18n.t('editor.body')}
      aria-invalid={problem !== null}
      value={bodyContent(body)}
      oninput={edit}></textarea>
    {#if problem}
      <p class="body-editor__problem" role="status">
        {i18n.t('editor.body_invalid_json', { detail: problem })}
      </p>
    {/if}
  {/if}
</div>

<style>
  .body-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .body-editor__file {
    display: flex;
    gap: var(--space-s);
    align-items: center;
  }

  .body-editor__file-field {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .body-editor__file-label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .body-editor__file-input {
    flex: 1;
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    font-family: var(--font-mono);
  }

  .body-editor__kinds {
    display: flex;
    gap: var(--space-xs);
  }

  .body-editor__kind {
    padding: var(--space-xs) var(--space-m);
    border-radius: 6px;
    color: var(--color-muted);
    cursor: pointer;
  }

  .body-editor__kind input {
    position: absolute;
    opacity: 0;
  }

  .body-editor__kind:focus-within {
    outline: 2px solid var(--color-accent);
  }

  .body-editor__kind--active {
    background: var(--color-raised);
    color: var(--color-text);
    font-weight: 500;
  }

  .body-editor__text {
    min-height: 160px;
    padding: var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-code-bg);
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--font-size-m);
    line-height: 1.6;
    resize: vertical;
  }

  .body-editor__note {
    margin: 0;
    color: var(--color-muted);
  }

  .body-editor__problem {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }
</style>
