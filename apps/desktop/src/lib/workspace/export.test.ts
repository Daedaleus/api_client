import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import DialogHost from '../../components/organisms/DialogHost.svelte';
import { FakeApi, shop } from '../../test/fake-api';
import { i18n } from '../i18n/i18n.svelte';
import { exportOptions } from './export-target';
import { Workspace } from './workspace.svelte';

async function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  api.answers.push({ type: 'collectionImported', collection: shop(), skipped: [] });
  api.file = 'shop.json';
  await workspace.importCollection('postman');
  api.commands.length = 0;
  return { api, workspace };
}

describe('collection export', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('suggests a safe file name in the save dialog', () => {
    expect(exportOptions('Shop: v2/beta\\x')).toEqual({
      title: 'Export as Postman collection',
      filters: [{ name: 'Postman collection (JSON)', extensions: ['json'] }],
      defaultPath: 'Shop_ v2_beta_x.postman_collection.json',
    });
    expect(exportOptions('  ').defaultPath).toBe('collection.postman_collection.json');
  });

  it('writes the file where the user chooses and lists what was left out', async () => {
    const { api, workspace } = await setup();
    render(DialogHost, { dialogs: workspace.dialogs });
    await workspace.exportCollection('c1');
    expect(api.commands).toEqual([]);
    expect(api.picks.at(-1)).toMatchObject({
      kind: 'save',
      options: { defaultPath: 'Shop.postman_collection.json' },
    });

    api.saveFile = 'C:\\out\\shop.json';
    api.answers.push({
      type: 'collectionExported',
      leftOut: [{ key: 'export.timeout', args: { item: 'Login' } }],
    });
    const exporting = workspace.exportCollection('c1');
    expect(await screen.findByRole('dialog')).toHaveTextContent('Export finished');
    expect(screen.getByText('“Login”: the time limit has no place there')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'OK' }));
    await exporting;
    expect(api.commands).toEqual([
      {
        type: 'exportCollection',
        collectionId: 'c1',
        format: 'postman',
        file: 'C:\\out\\shop.json',
      },
    ]);
  });

  it('needs no dialog when everything fits and reports errors', async () => {
    const { api, workspace } = await setup();
    api.saveFile = 'out.json';
    api.answers.push({ type: 'collectionExported', leftOut: [] });
    await workspace.exportCollection('c1');
    expect(workspace.dialogs.current).toBeNull();

    api.answers.push({ fail: { key: 'error.store_io', args: { detail: 'disk full' } } });
    await workspace.exportCollection('c1');
    expect(workspace.collections.error?.key).toBe('error.store_io');

    api.answers.push({ type: 'collectionClosed', collectionId: 'c1' });
    await workspace.exportCollection('c1');
    expect(workspace.collections.error?.args.detail).toBe('unexpected answer collectionClosed');

    const before = api.commands.length;
    await workspace.exportCollection('nope');
    expect(api.commands.length).toBe(before);
  });
});
