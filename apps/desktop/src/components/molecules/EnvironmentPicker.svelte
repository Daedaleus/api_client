<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { EnvironmentDraft } from '../../lib/types';

  interface Props {
    environments: EnvironmentDraft[];
    /** The chosen environment's id, `null` for none. */
    chosen: string | null;
    onchoose: (environmentId: string | null) => void;
    onedit: (environmentId: string) => void;
    onnew: () => void;
  }

  let { environments, chosen, onchoose, onedit, onnew }: Props = $props();

  /** Value of the "no environment" option (ids are never empty). */
  const NONE = '';

  function change(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    onchoose(value === NONE ? null : value);
  }
</script>

<div class="environment-picker">
  <label class="environment-picker__field">
    <span class="environment-picker__label">{i18n.t('env.label')}</span>
    <select class="environment-picker__select" value={chosen ?? NONE} onchange={change}>
      <option value={NONE}>{i18n.t('env.none')}</option>
      {#each environments as environment (environment.id)}
        <option value={environment.id}>{environment.name}</option>
      {/each}
    </select>
  </label>
  {#if chosen}
    <IconButton icon="edit" label={i18n.t('env.edit')} onclick={() => onedit(chosen)} />
  {/if}
  <IconButton icon="plus" label={i18n.t('env.new')} onclick={onnew} />
</div>

<style>
  .environment-picker {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .environment-picker__field {
    display: flex;
    align-items: center;
    gap: var(--space-s);
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .environment-picker__select {
    height: 28px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-surface);
    color: var(--color-text);
  }
</style>
