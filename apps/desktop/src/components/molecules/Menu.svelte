<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../atoms/Icon.svelte';
  import { fit, isNavigationKey, step, type MenuEntry, type MenuItem } from '../../lib/menu';
  import Menu from './Menu.svelte';

  interface Props {
    items: MenuItem[];
    /** Where the menu opens (window coordinates); moved to fit the window. */
    x: number;
    y: number;
    /** Accessible name. */
    label: string;
    /**
     * Closes the whole menu; `restoreFocus` after a choice or Escape, so
     * keyboard users return to where they were.
     */
    onclose: (restoreFocus: boolean) => void;
    /** Submenus only: back to the parent entry (ArrowLeft, Escape). */
    onback?: () => void;
    /** Clicks on this element (the menu's button) do not count as outside. */
    anchor?: HTMLElement;
  }

  let { items, x, y, label, onclose, onback, anchor }: Props = $props();

  let element = $state<HTMLUListElement>();
  let buttons = $state<HTMLButtonElement[]>([]);
  let position = $state({ left: 0, top: 0 });
  /** The open submenu: index of its entry and where it opens. */
  let submenu = $state<{ index: number; x: number; y: number } | null>(null);

  let entries = $derived(
    items.flatMap((item, index) => (item.kind === 'entry' ? [{ item, index }] : [])),
  );

  onMount(() => {
    const rect = element!.getBoundingClientRect();
    position = fit(x, y, rect.width, rect.height, {
      width: window.innerWidth,
      height: window.innerHeight,
    });
    buttons[0]?.focus();
  });

  function focused(): number {
    return buttons.findIndex((button) => button === document.activeElement);
  }

  function openSubmenu(position: number) {
    const button = buttons[position];
    if (!button) return;
    const rect = button.getBoundingClientRect();
    submenu = { index: position, x: rect.right - 2, y: rect.top - 4 };
  }

  function choose(item: MenuEntry, position: number) {
    if (item.items) {
      openSubmenu(position);
      return;
    }
    onclose(true);
    item.onSelect?.();
  }

  function keydown(event: KeyboardEvent) {
    // Keys inside an open submenu are handled there.
    if (submenu !== null && event.target !== element && !buttons.includes(event.target as never))
      return;
    const current = focused();
    if (isNavigationKey(event.key)) {
      event.preventDefault();
      buttons[step(current, event.key, buttons.length)]?.focus();
    } else if (event.key === 'ArrowRight' && entries[current]?.item.items) {
      event.preventDefault();
      openSubmenu(current);
    } else if (event.key === 'ArrowLeft' && onback) {
      event.preventDefault();
      onback();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      if (onback) onback();
      else onclose(true);
    } else if (event.key === 'Tab') {
      onclose(false);
      return;
    } else {
      return;
    }
    // Dialogs behind the menu must not see these keys.
    event.stopPropagation();
  }

  function back(position: number) {
    submenu = null;
    buttons[position]?.focus();
  }

  function pointerdown(event: PointerEvent) {
    if (onback) return;
    const target = event.target as Node;
    if (!element?.contains(target) && !anchor?.contains(target)) onclose(false);
  }
</script>

<svelte:window onpointerdown={pointerdown} onresize={() => onclose(false)} />

<ul
  bind:this={element}
  class="menu"
  role="menu"
  aria-label={label}
  style:left="{position.left}px"
  style:top="{position.top}px"
  onkeydown={keydown}
>
  {#each items as item, index (index)}
    {#if item.kind === 'separator'}
      <li class="menu__separator" role="separator"></li>
    {:else}
      {@const position = entries.findIndex((entry) => entry.index === index)}
      <li class="menu__entry" role="none">
        <button
          bind:this={buttons[position]}
          type="button"
          class="menu__item"
          class:menu__item--open={submenu?.index === position}
          role="menuitem"
          tabindex="-1"
          data-item={item.id}
          aria-haspopup={item.items ? 'menu' : undefined}
          aria-expanded={item.items ? submenu?.index === position : undefined}
          onclick={() => choose(item, position)}
          onmouseenter={() => {
            buttons[position]?.focus();
            if (item.items) openSubmenu(position);
            else submenu = null;
          }}
        >
          <span class="menu__label">{item.label}</span>
          {#if item.shortcut}
            <span class="menu__shortcut">{item.shortcut}</span>
          {/if}
          {#if item.items}
            <span class="menu__arrow"><Icon name="chevron" size={12} /></span>
          {/if}
        </button>
        {#if item.items && submenu?.index === position}
          <Menu
            items={item.items}
            x={submenu.x}
            y={submenu.y}
            label={item.label}
            {onclose}
            onback={() => back(position)}
          />
        {/if}
      </li>
    {/if}
  {/each}
</ul>

<style>
  .menu {
    position: fixed;
    z-index: 20;
    min-width: 200px;
    margin: 0;
    padding: var(--space-xs);
    list-style: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }

  .menu__item {
    display: flex;
    align-items: center;
    gap: var(--space-m);
    width: 100%;
    padding: var(--space-s) var(--space-m);
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }

  .menu__item:focus,
  .menu__item--open {
    outline: none;
    background: var(--color-raised);
  }

  .menu__item:focus-visible {
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }

  .menu__label {
    flex: 1;
  }

  .menu__shortcut {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .menu__arrow {
    display: inline-flex;
    color: var(--color-muted);
  }

  .menu__separator {
    height: 1px;
    margin: var(--space-xs) var(--space-s);
    background: var(--color-border);
  }
</style>
