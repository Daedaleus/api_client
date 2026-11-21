import { describe, expect, it } from 'vitest';

import { FakeApi, draft, memoryStorage, shop } from '../../test/fake-api';
import { Workspace } from './workspace.svelte';

function setup() {
  const api = new FakeApi();
  const workspace = new Workspace(api, memoryStorage());
  workspace.collections.apply(shop());
  return { api, workspace };
}

describe('cURL', () => {
  it('builds the command for a tab exactly as it would be sent', async () => {
    const { api, workspace } = setup();
    await workspace.openRequest('c1', 'r1');
    const tab = workspace.tabs.active!;
    workspace.environments.choose('c1', 'e1');
    api.queryAnswers.push({ type: 'curl', command: "curl 'https://x'" });
    expect(await workspace.curlFor(tab)).toBe("curl 'https://x'");
    expect(api.queries.at(-1)).toMatchObject({
      type: 'curl',
      url: 'https://example.com/r1',
      context: { collectionId: 'c1', requestId: 'r1', environmentId: 'e1' },
    });

    api.queryAnswers.push({ fail: { key: 'error.variable_unknown', args: { name: 'x' } } });
    expect(await workspace.curlFor(tab)).toBeNull();
    expect(tab.error?.key).toBe('error.variable_unknown');

    api.queryAnswers.push({ type: 'recipients', recipients: { own: 'a', recipients: [] } });
    expect(await workspace.curlFor(tab)).toBeNull();
    expect(tab.error?.args.detail).toBe('unexpected answer recipients');
  });

  it('opens a pasted command as a new tab and says what was left out', async () => {
    const { api, workspace } = setup();
    const curl = workspace.curl;
    await curl.import();
    curl.openImport();
    curl.importing!.text = 'curl https://x.example --compressed';

    api.queryAnswers.push({ fail: { key: 'error.curl_invalid', args: { detail: 'no URL' } } });
    await curl.import();
    expect(curl.importing?.error?.key).toBe('error.curl_invalid');

    api.queryAnswers.push({
      type: 'parsedCurl',
      request: draft('', 'cURL', { url: 'https://x.example' }),
      ignored: ['--compressed', '-L'],
    });
    await curl.import();
    expect(api.queries.at(-1)).toEqual({
      type: 'parseCurl',
      text: 'curl https://x.example --compressed',
    });
    expect(curl.importing).toBeNull();
    const tab = workspace.tabs.active!;
    expect(tab.saved).toBeNull();
    expect(tab.draft.url).toBe('https://x.example');
    expect(tab.notice).toEqual({ key: 'curl.ignored', args: { options: '--compressed -L' } });

    curl.openImport();
    api.queryAnswers.push({ type: 'parsedCurl', request: draft('', 'cURL'), ignored: [] });
    await curl.import();
    expect(workspace.tabs.active!.notice).toBeNull();

    curl.openImport();
    api.queryAnswers.push({ type: 'curl', command: 'x' });
    await curl.import();
    expect(curl.importing?.error?.args.detail).toBe('unexpected answer curl');
    curl.closeImport();
    expect(curl.importing).toBeNull();
  });
});
