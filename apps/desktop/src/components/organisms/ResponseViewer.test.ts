import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { BodyView, RunOutcome } from '../../lib/types';
import ResponseViewer from './ResponseViewer.svelte';

function completed(
  body: Partial<BodyView> = {},
  status = 200,
): Extract<RunOutcome, { kind: 'completed' }> {
  return {
    kind: 'completed',
    status,
    reason: status === 200 ? 'OK' : null,
    statusClass: status === 200 ? 'success' : 'clientError',
    elapsedMs: 142,
    sizeBytes: 2048,
    headers: [{ name: 'content-type', value: 'application/json' }],
    body: { text: '{\n  "a": 1\n}', pretty: true, truncated: false, binary: false, ...body },
    logs: [],
    scriptError: null,
    tests: [],
  };
}

describe('ResponseViewer', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('invites to send when there is no response yet', () => {
    render(ResponseViewer, { outcome: null, sending: false });
    expect(screen.getByText('No response yet. Enter a URL and send.')).toBeInTheDocument();
  });

  it('shows status, time, size and body', () => {
    render(ResponseViewer, { outcome: completed(), sending: false });
    expect(screen.getByText(/200\s+OK/)).toBeInTheDocument();
    expect(screen.getByText('142 ms')).toBeInTheDocument();
    expect(screen.getByText('2.0 KB')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Body' })).toHaveTextContent('"a": 1');
  });

  it('switches to the headers', async () => {
    render(ResponseViewer, { outcome: completed(), sending: false });
    await userEvent.click(screen.getByRole('tab', { name: /Headers/ }));
    expect(screen.getByRole('tab', { name: /Headers/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('application/json')).toBeInTheDocument();
  });

  it('explains empty, binary and truncated bodies', () => {
    const { rerender } = render(ResponseViewer, {
      outcome: completed({ text: '', pretty: false }, 404),
      sending: false,
    });
    expect(screen.getByText('No content.')).toBeInTheDocument();
    expect(screen.getByText('404')).toBeInTheDocument();

    rerender({ outcome: completed({ text: '', binary: true }), sending: false });
    expect(screen.getByText('Binary content (2.0 KB), no preview.')).toBeInTheDocument();

    rerender({ outcome: completed({ text: 'abc', truncated: true }), sending: false });
    expect(screen.getByText('Preview truncated to the first 3 B.')).toBeInTheDocument();
  });

  it('shows why a request failed, in the chosen language', () => {
    i18n.setLocale('de');
    render(ResponseViewer, {
      outcome: { kind: 'failed', error: { key: 'error.timeout', args: {} }, logs: [], tests: [] },
      sending: false,
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Request fehlgeschlagen');
    expect(screen.getByRole('alert')).toHaveTextContent('Zeitüberschreitung');
  });

  it('shows what scripts logged and why the script after the response failed', async () => {
    render(ResponseViewer, {
      outcome: {
        ...completed(),
        logs: ['pre ran', 'status 200'],
        scriptError: { key: 'error.script_post_response', args: { detail: 'Error: boom' } },
      },
      sending: false,
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Post-response script: Error: boom');
    await userEvent.click(screen.getByRole('tab', { name: /Log/ }));
    expect(screen.getByText(/pre ran/)).toHaveTextContent('pre ran status 200');
  });

  it('shows the log of a failed run and no log tab without logs', () => {
    render(ResponseViewer, {
      outcome: {
        kind: 'failed',
        error: { key: 'error.script_pre_request', args: { detail: 'stopped after 5 s' } },
        logs: ['started'],
        tests: [],
      },
      sending: false,
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Pre-request script: stopped after 5 s');
    expect(screen.getByLabelText('Log')).toHaveTextContent('started');
  });

  it('falls back to the body when the log tab disappears', async () => {
    const view = render(ResponseViewer, {
      outcome: { ...completed(), logs: ['x'] },
      sending: false,
    });
    await userEvent.click(screen.getByRole('tab', { name: /Log/ }));
    await view.rerender({ outcome: completed(), sending: false });
    expect(screen.queryByRole('tab', { name: /Log/ })).not.toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Body' })).toHaveAttribute('aria-selected', 'true');
  });

  it('lists the tests with what failed and counts them on the tab', async () => {
    render(ResponseViewer, {
      outcome: {
        ...completed(),
        tests: [
          { name: 'status is 200', passed: true, message: null },
          { name: 'has items', passed: false, message: 'expected 0 to be greater than 0' },
        ],
      },
      sending: false,
    });
    const tab = screen.getByRole('tab', { name: /Tests/ });
    expect(tab).toHaveTextContent('1/2');
    await userEvent.click(tab);
    const list = screen.getByRole('list', { name: 'Tests' });
    expect(list).toHaveTextContent('status is 200');
    expect(list).toHaveTextContent('expected 0 to be greater than 0');
    expect(screen.getAllByRole('img', { name: 'failed' })).toHaveLength(1);
    expect(screen.getAllByRole('img', { name: 'passed' })).toHaveLength(1);
  });

  it('shows the tests of a failed run below the error', () => {
    render(ResponseViewer, {
      outcome: {
        kind: 'failed',
        error: { key: 'error.timeout', args: {} },
        logs: [],
        tests: [{ name: 'before', passed: true, message: null }],
      },
      sending: false,
    });
    expect(screen.getByRole('list', { name: 'Tests' })).toHaveTextContent('before');
  });

  it('shows a body with a huge line as plain text', () => {
    const line = 'x'.repeat(30_000);
    render(ResponseViewer, { outcome: completed({ text: line, pretty: false }), sending: false });
    expect(screen.queryByRole('textbox', { name: 'Body' })).not.toBeInTheDocument();
    expect(screen.getByText(line)).toBeInTheDocument();
  });
});
