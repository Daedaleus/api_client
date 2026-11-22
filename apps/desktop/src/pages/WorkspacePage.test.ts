import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../lib/i18n/i18n.svelte';
import { FakeApi, changed, memoryStorage, shop } from '../test/fake-api';
import WorkspacePage from './WorkspacePage.svelte';

/** The tabs of open requests (not the sections inside the editor). */
const openTabs = () =>
  within(screen.getByRole('tablist', { name: 'Open requests' })).getAllByRole('tab');

/** The page with `Shop` restored from the last session. */
async function setup() {
  const api = new FakeApi();
  api.answers.push(changed(shop()));
  const storage = memoryStorage({ openCollections: JSON.stringify(['C:\\apis\\shop']) });
  render(WorkspacePage, { api, storage });
  await screen.findByText('Health');
  return { api };
}

describe('WorkspacePage', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('shows the restored collection as a tree', async () => {
    await setup();
    const sidebar = screen.getByRole('complementary', { name: 'Collections' });
    expect(within(sidebar).getByText('Shop')).toBeInTheDocument();
    expect(within(sidebar).getByText('Login')).toBeInTheDocument();

    await userEvent.click(within(sidebar).getByRole('button', { name: 'Auth' }));
    expect(within(sidebar).queryByText('Login')).not.toBeInTheDocument();
  });

  it('opens a request in a tab, edits and saves it with Ctrl+S', async () => {
    const { api } = await setup();
    await userEvent.click(screen.getByRole('button', { name: 'GET Health' }));
    const name = await screen.findByLabelText('Name');
    expect(name).toHaveValue('Request r2');
    expect(screen.getByRole('button', { name: 'GET Health' })).toHaveAttribute(
      'aria-current',
      'page',
    );

    const url = screen.getByLabelText('URL');
    await userEvent.clear(url);
    await userEvent.type(url, 'https://example.com/v2');
    expect(screen.getByRole('img', { name: 'unsaved changes' })).toBeInTheDocument();

    api.answers.push(changed(shop()));
    await fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    await waitFor(() => expect(api.commands.at(-1)?.type).toBe('saveRequest'));
    expect(api.commands.at(-1)).toMatchObject({
      request: { url: 'https://example.com/v2', name: 'Request r2' },
    });
    await waitFor(() =>
      expect(screen.queryByRole('img', { name: 'unsaved changes' })).not.toBeInTheDocument(),
    );
  });

  it('creates a request through the name dialog', async () => {
    const { api } = await setup();
    const sidebar = screen.getByRole('complementary', { name: 'Collections' });
    const collection = within(sidebar).getByRole('region', { name: 'Shop' });
    await userEvent.click(within(collection).getAllByRole('button', { name: 'New request' })[0]!);

    const dialog = await screen.findByRole('dialog', { name: 'New request' });
    api.answers.push(changed(shop(), 'r2'));
    await userEvent.type(within(dialog).getByLabelText('Name'), 'Status{Enter}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(api.commands.at(-1)).toEqual({
      type: 'createRequest',
      collectionId: 'c1',
      parentId: 'c1',
      name: 'Status',
    });
  });

  it('confirms before deleting and can be cancelled with Escape', async () => {
    const { api } = await setup();
    await userEvent.click(screen.getByRole('button', { name: 'Actions for “Auth”' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Delete' }));
    const dialog = await screen.findByRole('dialog', { name: 'Delete' });
    expect(dialog).toHaveTextContent('Move folder “Auth” with all its contents to the trash?');
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(api.commands.filter((command) => command.type === 'deleteItem')).toHaveLength(0);
  });

  it('keeps scratch tabs for quick requests', async () => {
    await setup();
    expect(screen.getByText(/Not saved/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'New tab' }));
    expect(openTabs()).toHaveLength(2);
    await userEvent.click(screen.getAllByRole('button', { name: 'Close tab' })[0]!);
    expect(openTabs()).toHaveLength(1);
  });

  it('opens new requests from the "New" menu and with Ctrl+N', async () => {
    await setup();
    expect(openTabs()).toHaveLength(1);
    await userEvent.click(screen.getByRole('button', { name: 'New' }));
    await userEvent.click(screen.getByRole('menuitem', { name: /New request/ }));
    expect(openTabs()).toHaveLength(2);
    await fireEvent.keyDown(window, { key: 'n', ctrlKey: true });
    expect(openTabs()).toHaveLength(3);
    await fireEvent.keyDown(window, { key: 'n', ctrlKey: true, shiftKey: true });
    expect(openTabs()).toHaveLength(3);
  });

  it('sends with Ctrl+Enter and focuses the environment with Ctrl+E', async () => {
    const { api } = await setup();
    await userEvent.click(screen.getByRole('button', { name: 'GET Health' }));
    await screen.findByLabelText('Name');
    expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute(
      'title',
      'Send (Ctrl+Enter)',
    );

    await fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(api.commands.at(-1)?.type).toBe('sendRequest'));

    await fireEvent.keyDown(window, { key: 'e', ctrlKey: true });
    expect(screen.getByRole('combobox', { name: 'Environment' })).toHaveFocus();
  });

  it('moves a request by dragging it in the tree', async () => {
    const { api } = await setup();
    const health = screen.getByRole('button', { name: 'GET Health' });
    const authRow = screen.getByRole('button', { name: 'Auth' }).closest('.tree-node__row')!;
    await fireEvent.pointerDown(health.closest('.tree-node__row')!, {
      button: 0,
      clientX: 10,
      clientY: 50,
    });
    await fireEvent.pointerMove(window, { clientX: 10, clientY: 20 });
    // Rows have no size in the test DOM: the top of the Auth row, before it.
    await fireEvent.pointerMove(authRow, { clientX: 10, clientY: 0 });
    expect(authRow).toHaveClass('tree-node__row--drop-before');
    api.answers.push(changed(shop()));
    await fireEvent.pointerUp(window);
    // The click that ends the drag does not open the request.
    await fireEvent.click(health);
    await waitFor(() =>
      expect(api.commands.at(-1)).toEqual({
        type: 'moveItem',
        collectionId: 'c1',
        itemId: 'r2',
        parentId: 'c1',
        index: 0,
      }),
    );
    expect(api.queries.filter((query) => query.type === 'request')).toEqual([]);
  });

  it('finds requests with Ctrl+K and filters the tree', async () => {
    const { api } = await setup();
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    const search = await screen.findByRole('combobox', { name: 'Find a request' });
    await userEvent.type(search, 'auth');
    const results = screen.getByRole('listbox');
    expect(
      within(results)
        .getAllByRole('option')
        .map((option) => option.textContent),
    ).toEqual([expect.stringContaining('Login')]);
    await userEvent.type(search, 'xyz');
    expect(screen.getByText('No request found.')).toBeInTheDocument();
    await userEvent.clear(search);
    await userEvent.type(search, 'e');
    await userEvent.keyboard('{ArrowDown}{ArrowUp}{ArrowDown}{Enter}');
    expect(screen.queryByRole('dialog', { name: 'Find a request' })).not.toBeInTheDocument();
    await waitFor(() => expect(api.queries.at(-1)).toMatchObject({ type: 'request' }));

    // Escape and a click outside close it.
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog', { name: 'Find a request' })).not.toBeInTheDocument();

    // The filter keeps matching requests and their folders, unfolded.
    const sidebar = screen.getByRole('complementary', { name: 'Collections' });
    await userEvent.click(within(sidebar).getByRole('button', { name: 'Auth' }));
    expect(within(sidebar).queryByText('Login')).not.toBeInTheDocument();
    await userEvent.type(
      within(sidebar).getByRole('searchbox', { name: 'Filter collections' }),
      'login',
    );
    expect(within(sidebar).getByText('Login')).toBeInTheDocument();
    expect(within(sidebar).queryByText('Health')).not.toBeInTheDocument();
  });

  it('opens a collection folder with its own button', async () => {
    const { api } = await setup();
    api.folder = 'C:\\apis\\billing';
    api.answers.push(changed(shop()));
    await userEvent.click(screen.getByRole('button', { name: 'Open collection …' }));
    await waitFor(() =>
      expect(api.commands.at(-1)).toEqual({ type: 'openCollection', path: api.folder }),
    );
    expect(api.picks.at(-1)?.kind).toBe('folder');
  });

  it('offers the actions of a collection by right click', async () => {
    await setup();
    const sidebar = screen.getByRole('complementary', { name: 'Collections' });
    await fireEvent.contextMenu(within(sidebar).getByText('Shop'));
    const menu = screen.getByRole('menu', { name: 'Actions for “Shop”' });
    await userEvent.click(
      within(menu).getByRole('menuitem', { name: 'Collection variables & auth' }),
    );
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
  });

  it('shows load problems and errors of the collection', async () => {
    const api = new FakeApi();
    api.answers.push(
      changed({
        ...shop(),
        problems: [{ key: 'error.store_damaged', args: { path: 'a.toml', detail: 'bad' } }],
      }),
    );
    render(WorkspacePage, {
      api,
      storage: memoryStorage({ openCollections: JSON.stringify(['x']) }),
    });
    expect(await screen.findByText('Not loaded (1):')).toBeInTheDocument();
    expect(screen.getByText(/“a.toml” is damaged/)).toBeInTheDocument();
  });

  it('lets error messages be closed', async () => {
    const api = new FakeApi();
    // The remembered collection cannot be opened any more.
    api.answers.push({ fail: { key: 'error.not_a_collection', args: { path: 'C:/gone' } } });
    render(WorkspacePage, {
      api,
      storage: memoryStorage({ openCollections: JSON.stringify(['C:/gone']) }),
    });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('gone');
    await userEvent.click(screen.getByRole('button', { name: 'Close message' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    // The error under the URL closes the same way.
    api.answers.push({ fail: { key: 'error.url_empty', args: {} } });
    await userEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('URL');
    await userEvent.click(screen.getByRole('button', { name: 'Close message' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('invites to open a collection when none is open', async () => {
    render(WorkspacePage, { api: new FakeApi(), storage: null });
    expect(await screen.findByText(/No collection open yet/)).toBeInTheDocument();
  });
});
