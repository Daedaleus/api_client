import { beforeEach, describe, expect, it } from 'vitest';

import { FakeApi, OK } from '../../test/fake-api';
import { i18n } from '../i18n/i18n.svelte';
import { responseOptions } from './export-target';
import { Workspace } from './workspace.svelte';

describe('saving a response', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('suggests a file name from the content type', () => {
    const named = (type: string) =>
      responseOptions([{ name: 'Content-Type', value: type }]).defaultPath;
    expect(named('application/json; charset=utf-8')).toBe('response.json');
    expect(named('application/problem+json')).toBe('response.json');
    expect(named('image/png')).toBe('response.png');
    expect(named('application/x-unknown')).toBe('response.bin');
    expect(responseOptions([]).defaultPath).toBe('response.bin');
    expect(responseOptions([]).title).toBe('Save response');
  });

  it('writes the body of the shown run where the user chooses', async () => {
    const api = new FakeApi();
    const workspace = new Workspace(api, null);
    const tab = workspace.tabs.active!;
    // Nothing to save yet, or a result from the history.
    await workspace.saveResponse(tab);
    tab.outcome = OK;
    await workspace.saveResponse(tab);
    expect(api.picks).toEqual([]);

    tab.outcomeRun = 7;
    await workspace.saveResponse(tab);
    expect(api.commands).toEqual([]);

    api.saveFile = 'C:\\out\\response.txt';
    api.answers.push({ type: 'responseSaved' });
    await workspace.saveResponse(tab);
    expect(api.commands.at(-1)).toEqual({
      type: 'saveResponseBody',
      runId: 7,
      file: 'C:\\out\\response.txt',
    });
    expect(tab.error).toBeNull();

    api.answers.push({ fail: { key: 'error.response_gone', args: {} } });
    await workspace.saveResponse(tab);
    expect(tab.error?.key).toBe('error.response_gone');
    api.answers.push({ type: 'settingsSaved' });
    await workspace.saveResponse(tab);
    expect(tab.error?.args.detail).toBe('unexpected answer settingsSaved');
  });
});
