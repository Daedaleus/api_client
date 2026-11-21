<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { ScriptsDraft } from '../../lib/types';

  let { scripts = $bindable() }: { scripts: ScriptsDraft } = $props();
</script>

<div class="scripts-editor">
  <p class="scripts-editor__hint">{i18n.t('scripts.hint')}</p>
  <label class="scripts-editor__field">
    <span class="scripts-editor__label">{i18n.t('scripts.pre_request')}</span>
    <textarea
      class="scripts-editor__input"
      spellcheck="false"
      rows="6"
      placeholder={i18n.t('scripts.placeholder_pre_request')}
      bind:value={scripts.preRequest}></textarea>
  </label>
  <label class="scripts-editor__field">
    <span class="scripts-editor__label">{i18n.t('scripts.post_response')}</span>
    <textarea
      class="scripts-editor__input"
      spellcheck="false"
      rows="6"
      placeholder={i18n.t('scripts.placeholder_post_response')}
      bind:value={scripts.postResponse}></textarea>
  </label>
</div>

<style>
  .scripts-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
  }

  .scripts-editor__hint {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
    line-height: 1.5;
  }

  .scripts-editor__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .scripts-editor__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
    font-weight: 600;
  }

  .scripts-editor__input {
    padding: var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-code-bg);
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    line-height: 1.5;
    resize: vertical;
    tab-size: 2;
  }
</style>
