<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { ScriptAssist } from '../../lib/script-assist';
  import type { ScriptsDraft } from '../../lib/types';
  import ScriptEditor from './ScriptEditor.svelte';

  interface Props {
    scripts: ScriptsDraft;
    /** Checker and completion for both scripts. */
    assist?: ScriptAssist | undefined;
  }

  let { scripts = $bindable(), assist }: Props = $props();
</script>

<div class="scripts-editor">
  <p class="scripts-editor__hint">{i18n.t('scripts.hint')}</p>
  <div class="scripts-editor__field">
    <span class="scripts-editor__label">{i18n.t('scripts.pre_request')}</span>
    <ScriptEditor
      bind:value={scripts.preRequest}
      label={i18n.t('scripts.pre_request')}
      placeholder={i18n.t('scripts.placeholder_pre_request')}
      {assist}
    />
  </div>
  <div class="scripts-editor__field">
    <span class="scripts-editor__label">{i18n.t('scripts.post_response')}</span>
    <ScriptEditor
      bind:value={scripts.postResponse}
      label={i18n.t('scripts.post_response')}
      placeholder={i18n.t('scripts.placeholder_post_response')}
      {assist}
    />
  </div>
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
</style>
