<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    type?: 'button' | 'submit';
    variant?: 'primary' | 'secondary';
    disabled?: boolean;
    onclick?: () => void;
    /** Tooltip, e.g. with the keyboard shortcut. */
    title?: string | undefined;
    children: Snippet;
  }

  let {
    type = 'button',
    variant = 'secondary',
    disabled = false,
    onclick,
    title,
    children,
  }: Props = $props();
</script>

<button {type} {disabled} {onclick} {title} class="button button--{variant}">
  {@render children()}
</button>

<style>
  .button {
    height: var(--control-height);
    padding: 0 var(--space-l);
    border-radius: var(--radius);
    border: 1px solid var(--color-border);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .button--primary {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .button--secondary {
    background: var(--color-raised);
  }

  .button:disabled {
    cursor: default;
    opacity: 0.6;
  }
</style>
