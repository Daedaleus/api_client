import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import { Workspace } from '../../lib/workspace/workspace.svelte';
import { FakeApi, draft } from '../../test/fake-api';
import CurlImportDialog from './CurlImportDialog.svelte';

function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, null);
  workspace.curl.openImport();
  render(CurlImportDialog, { curl: workspace.curl });
  return { api, workspace };
}

describe('CurlImportDialog', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('opens a pasted command, showing errors first', async () => {
    const { api, workspace } = setup();
    const open = screen.getByRole('button', { name: 'Open' });
    expect(open).toBeDisabled();
    await userEvent.click(screen.getByLabelText('cURL command'));
    await userEvent.paste('curl https://x.example');

    api.queryAnswers.push({ fail: { key: 'error.curl_invalid', args: { detail: 'no URL' } } });
    await userEvent.click(open);
    expect(await screen.findByRole('alert')).toHaveTextContent('no usable cURL command: no URL');

    api.queryAnswers.push({ type: 'parsedCurl', request: draft('', 'cURL'), ignored: [] });
    await userEvent.click(open);
    await vi.waitFor(() => expect(workspace.curl.importing).toBeNull());
  });

  it('closes with Escape or Cancel', async () => {
    const { workspace } = setup();
    await userEvent.click(screen.getByLabelText('cURL command'));
    await userEvent.keyboard('{Escape}');
    expect(workspace.curl.importing).toBeNull();
    workspace.curl.openImport();
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(workspace.curl.importing).toBeNull();
  });
});
