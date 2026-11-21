import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { ScriptsDraft } from '../../lib/types';
import ScriptsEditor from './ScriptsEditor.svelte';

describe('ScriptsEditor', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('edits the scripts before the request and after the response', async () => {
    const scripts = $state<{ value: ScriptsDraft }>({
      value: { preRequest: '', postResponse: '' },
    });
    render(ScriptsEditor, {
      get scripts() {
        return scripts.value;
      },
      set scripts(next) {
        scripts.value = next;
      },
    });
    expect(screen.getByText(/TypeScript or JavaScript/)).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Before the request'), "log('pre')");
    await userEvent.type(screen.getByLabelText('After the response'), 'log(res.status)');
    expect(scripts.value).toEqual({ preRequest: "log('pre')", postResponse: 'log(res.status)' });
  });
});
