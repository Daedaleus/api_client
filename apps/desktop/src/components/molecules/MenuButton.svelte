<script lang="ts">
  import Icon, { type IconName } from '../atoms/Icon.svelte';
  import type { MenuItem } from '../../lib/menu';
  import Menu from './Menu.svelte';

  interface Props {
    items: MenuItem[];
    /** Accessible name and tooltip (also of the menu). */
    label: string;
    /** An icon-only button … */
    icon?: IconName;
    /** … or one with this text. */
    text?: string;
  }

  let { items, label, icon, text }: Props = $props();

  let trigger = $state<HTMLButtonElement>();
  let open = $state<{ x: number; y: number } | null>(null);

  function show() {
    const rect = trigger!.getBoundingClientRect();
    open = { x: rect.left, y: rect.bottom + 4 };
  }

  function close(restoreFocus: boolean) {
    open = null;
    if (restoreFocus) trigger?.focus();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' && open === null) {
      event.preventDefault();
      show();
    }
  }
</script>

<button
  bind:this={trigger}
  type="button"
  class="menu-button"
  class:menu-button--icon={!text}
  aria-haspopup="menu"
  aria-expanded={open !== null}
  aria-label={text ? undefined : label}
  title={label}
  onclick={() => (open ? close(false) : show())}
  onkeydown={keydown}
>
  {#if icon}<Icon name={icon} />{/if}
  {#if text}
    <span>{text}</span>
    <span class="menu-button__caret"><Icon name="chevron" size={10} /></span>
  {/if}
</button>
{#if open}
  <Menu {items} x={open.x} y={open.y} {label} onclose={close} anchor={trigger} />
{/if}

<style>
  .menu-button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-raised);
    color: var(--color-text);
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }

  .menu-button--icon {
    width: 26px;
    height: 26px;
    padding: 0;
    justify-content: center;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-muted);
  }

  .menu-button:hover,
  .menu-button[aria-expanded='true'] {
    background: var(--color-raised);
    color: var(--color-text);
  }

  .menu-button__caret {
    display: inline-flex;
    transform: rotate(90deg);
  }
</style>
