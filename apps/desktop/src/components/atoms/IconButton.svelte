<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';

  interface Props {
    icon: IconName;
    /** Accessible name and tooltip. */
    label: string;
    onclick: () => void;
  }

  let { icon, label, onclick }: Props = $props();
</script>

<button type="button" class="icon-button" aria-label={label} title={label} {onclick}>
  <Icon name={icon} />
</button>

<style>
  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-muted);
    cursor: pointer;
  }

  .icon-button:hover {
    background: var(--color-raised);
    color: var(--color-text);
  }
</style>
