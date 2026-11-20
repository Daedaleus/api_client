import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { BodyView, RunOutcome } from '../../lib/types';
import ResponseViewer from './ResponseViewer.svelte';

function completed(body: Partial<BodyView> = {}, status = 200): RunOutcome {
  return {
    kind: 'completed',
    status,
    reason: status === 200 ? 'OK' : null,
    statusClass: status === 200 ? 'success' : 'clientError',
    elapsedMs: 142,
    sizeBytes: 2048,
    headers: [{ name: 'content-type', value: 'application/json' }],
    body: { text: '{\n  "a": 1\n}', pretty: true, truncated: false, binary: false, ...body },
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
    expect(screen.getByText(/"a": 1/)).toBeInTheDocument();
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
      outcome: { kind: 'failed', error: { key: 'error.timeout', args: {} } },
      sending: false,
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Request fehlgeschlagen');
    expect(screen.getByRole('alert')).toHaveTextContent('Zeitüberschreitung');
  });
});
