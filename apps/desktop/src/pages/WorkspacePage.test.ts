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

  it('invites to open a collection when none is open', async () => {
    render(WorkspacePage, { api: new FakeApi(), storage: null });
    expect(await screen.findByText(/No collection open yet/)).toBeInTheDocument();
  });
});
