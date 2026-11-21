<script lang="ts">
  import type { Snippet } from 'svelte';

  import type { MenuItem } from '../../lib/menu';
  import Menu from './Menu.svelte';

  interface Props {
    /** The entries, built when the menu opens. */
    items: () => MenuItem[];
    /** Accessible name of the menu. */
    label: string;
    children: Snippet;
  }

  let { items, label, children }: Props = $props();

  let open = $state<{ x: number; y: number; entries: MenuItem[] } | null>(null);
  /** Gets the focus back when the menu closes after keyboard use. */
  let returnTo: HTMLElement | null = null;

  function show(x: number, y: number) {
    returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    open = { x, y, entries: items() };
  }

  function contextmenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    show(event.clientX, event.clientY);
  }

  /** The context menu key and Shift+F10 open it at the focused element. */
  function keydown(event: KeyboardEvent) {
    if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10')) return;
    event.preventDefault();
    event.stopPropagation();
    const target = event.target as HTMLElement;
    const rect = target.getBoundingClientRect();
    show(rect.left, rect.bottom);
  }

  function close(restoreFocus: boolean) {
    open = null;
    if (restoreFocus) returnTo?.focus();
  }
</script>

<!-- The wrapper only catches events of the focusable elements inside it. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="context-menu" oncontextmenu={contextmenu} onkeydown={keydown}>
  {@render children()}
</div>
{#if open}
  <Menu items={open.entries} x={open.x} y={open.y} {label} onclose={close} />
{/if}

<style>
  .context-menu {
    display: contents;
  }
</style>
