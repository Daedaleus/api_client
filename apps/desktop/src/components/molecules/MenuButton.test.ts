import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { entry, separator } from '../../lib/menu';
import ContextMenu from './ContextMenu.test.svelte';
import MenuButton from './MenuButton.svelte';

function setup() {
  const actions = { request: vi.fn(), postman: vi.fn(), curl: vi.fn() };
  const items = [
    entry('request', 'New request', actions.request, 'Ctrl+N'),
    separator,
    entry('import', 'Import', [
      entry('postman', 'Postman', actions.postman),
      entry('curl', 'cURL', actions.curl),
    ]),
  ];
  render(MenuButton, { items, label: 'New things', text: 'New' });
  return { actions, trigger: screen.getByRole('button', { name: 'New' }) };
}

describe('MenuButton', () => {
  it('opens a menu with the first entry focused and closes with Escape', async () => {
    const { trigger } = setup();
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(trigger);
    expect(screen.getByRole('menu', { name: 'New things' })).toBeInTheDocument();
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    const [request, imports] = screen.getAllByRole('menuitem');
    expect(request).toHaveFocus();
    expect(request).toHaveTextContent('Ctrl+N');
    expect(screen.getByRole('separator')).toBeInTheDocument();

    await userEvent.keyboard('{ArrowDown}');
    expect(imports).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(request).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}');
    expect(imports).toHaveFocus();

    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('walks into submenus by keyboard and acts on Enter', async () => {
    const { actions, trigger } = setup();
    trigger.focus();
    await userEvent.keyboard('{ArrowDown}');
    await userEvent.keyboard('{End}');
    const imports = screen.getByRole('menuitem', { name: 'Import' });
    expect(imports).toHaveFocus();
    expect(imports).toHaveAttribute('aria-haspopup', 'menu');

    await userEvent.keyboard('{ArrowRight}');
    expect(imports).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('menuitem', { name: 'Postman' })).toHaveFocus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(screen.queryByRole('menuitem', { name: 'Postman' })).not.toBeInTheDocument();
    expect(imports).toHaveFocus();

    await userEvent.keyboard('{Enter}');
    await userEvent.keyboard('{Home}{End}{Enter}');
    expect(actions.curl).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('opens submenus on hover and closes on outside clicks, Tab and the button', async () => {
    const { actions, trigger } = setup();
    await userEvent.click(trigger);
    await userEvent.hover(screen.getByRole('menuitem', { name: 'Import' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Postman' }));
    expect(actions.postman).toHaveBeenCalledOnce();

    await userEvent.click(trigger);
    await userEvent.hover(screen.getByRole('menuitem', { name: 'Import' }));
    await userEvent.hover(screen.getByRole('menuitem', { name: /New request/ }));
    expect(screen.queryByRole('menuitem', { name: 'Postman' })).not.toBeInTheDocument();
    await userEvent.click(document.body);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();

    await userEvent.click(trigger);
    await userEvent.click(trigger);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();

    await userEvent.click(trigger);
    await userEvent.keyboard('{Tab}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();

    await userEvent.click(trigger);
    await fireEvent(window, new Event('resize'));
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(actions.request).not.toHaveBeenCalled();
  });

  it('can be an icon button', () => {
    render(MenuButton, { items: [], label: 'More', icon: 'more' });
    expect(screen.getByRole('button', { name: 'More' })).toHaveAttribute('title', 'More');
  });
});

describe('ContextMenu', () => {
  it('opens on right click and with Shift+F10 at the focused element', async () => {
    const act = vi.fn();
    render(ContextMenu, { items: () => [entry('rename', 'Rename', act)] });
    const row = screen.getByRole('button', { name: 'Row' });
    await fireEvent.contextMenu(row, { clientX: 30, clientY: 40 });
    expect(screen.getByRole('menu', { name: 'Row actions' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();

    row.focus();
    await userEvent.keyboard('{Shift>}{F10}{/Shift}');
    expect(screen.getByRole('menuitem', { name: 'Rename' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(row).toHaveFocus();

    await userEvent.keyboard('{ContextMenu}');
    await userEvent.keyboard('{Enter}');
    expect(act).toHaveBeenCalledOnce();
    await userEvent.keyboard('a');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
});
