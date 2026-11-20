import { describe, expect, it } from 'vitest';

import type { Api } from './api';
import { RequestController, toUserText } from './request.svelte';
import type { AppEvent, Command, CommandAck, RunOutcome } from './types';

const OK: RunOutcome = {
  kind: 'completed',
  status: 200,
  reason: 'OK',
  statusClass: 'success',
  elapsedMs: 12,
  sizeBytes: 2,
  headers: [],
  body: { text: 'ok', pretty: false, truncated: false, binary: false },
};

/** Fake backend: acknowledges with increasing run ids, events are emitted by the test. */
class FakeApi implements Api {
  commands: Command[] = [];
  handler: ((event: AppEvent) => void) | null = null;
  unsubscribed = false;
  nextRun = 1;
  failWith: unknown = null;
  /** Emitted before `execute` resolves, to simulate a very fast response. */
  emitBeforeAck: RunOutcome | null = null;

  async execute(command: Command): Promise<CommandAck> {
    this.commands.push(command);
    if (this.failWith !== null) throw this.failWith;
    const runId = this.nextRun++;
    if (this.emitBeforeAck) this.emit(runId, this.emitBeforeAck);
    return { type: 'runStarted', runId };
  }

  async onEvent(handler: (event: AppEvent) => void) {
    this.handler = handler;
    return () => {
      this.unsubscribed = true;
    };
  }

  emit(runId: number, outcome: RunOutcome) {
    this.handler?.({ type: 'runFinished', runId, outcome });
  }
}

async function setup() {
  const api = new FakeApi();
  const request = new RequestController(api);
  const stop = await request.connect();
  return { api, request, stop };
}

describe('RequestController', () => {
  it('sends what the user entered and shows the matching result', async () => {
    const { api, request } = await setup();
    request.method = 'POST';
    request.url = 'example.com';

    await request.send();
    expect(api.commands).toEqual([{ type: 'sendRequest', method: 'POST', url: 'example.com' }]);
    expect(request.sending).toBe(true);

    api.emit(1, OK);
    expect(request.sending).toBe(false);
    expect(request.outcome).toEqual(OK);
  });

  it('ignores results of older runs', async () => {
    const { api, request } = await setup();
    await request.send();
    await request.send();
    api.emit(1, OK);
    expect(request.pendingRun).toBe(2);
    expect(request.outcome).toBeNull();
  });

  it('handles results that arrive before the acknowledgement', async () => {
    const { api, request } = await setup();
    api.emitBeforeAck = OK;
    await request.send();
    expect(request.sending).toBe(false);
    expect(request.outcome).toEqual(OK);
  });

  it('shows why a send could not start', async () => {
    const { api, request } = await setup();
    api.failWith = { key: 'error.url_empty', args: {} };
    await request.send();
    expect(request.error).toEqual({ key: 'error.url_empty', args: {} });
    expect(request.sending).toBe(false);

    api.failWith = null;
    await request.send();
    expect(request.error).toBeNull();
  });

  it('unsubscribes via the returned function', async () => {
    const { api, stop } = await setup();
    stop();
    expect(api.unsubscribed).toBe(true);
  });
});

describe('toUserText', () => {
  it('keeps user texts and wraps anything else', () => {
    expect(toUserText({ key: 'a', args: {} })).toEqual({ key: 'a', args: {} });
    expect(toUserText(new Error('boom'))).toEqual({
      key: 'error.unexpected',
      args: { detail: 'boom' },
    });
    expect(toUserText('command not found')).toEqual({
      key: 'error.unexpected',
      args: { detail: 'command not found' },
    });
  });
});
