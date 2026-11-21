// A fake backend for UI tests: records what the UI asks for and answers
// from a queue, or with sensible defaults.

import type { Api, PickOptions } from '../lib/api';
import type {
  AppEvent,
  CollectionView,
  Command,
  CommandAck,
  Query,
  QueryResult,
  RequestDraft,
  RunOutcome,
} from '../lib/types';

/** A queued answer: an acknowledgement, or a failure to reject with. */
export type Answer = CommandAck | { fail: unknown };
export type QueryAnswer = QueryResult | { fail: unknown };

export const OK: RunOutcome = {
  kind: 'completed',
  status: 200,
  reason: 'OK',
  statusClass: 'success',
  elapsedMs: 12,
  sizeBytes: 2,
  headers: [],
  body: { text: 'ok', pretty: false, truncated: false, binary: false },
  logs: [],
  scriptError: null,
};

/** `Shop` with folder `Auth` (request `Login`) and request `Health`. */
export function shop(): CollectionView {
  return {
    id: 'c1',
    name: 'Shop',
    root: 'C:\\apis\\shop',
    problems: [],
    variables: [],
    auth: { kind: 'inherit' },
    certificateCheck: 'inherit',
    environments: [
      {
        id: 'e1',
        name: 'Dev',
        readOnly: false,
        variables: [
          { name: 'baseUrl', value: 'http://localhost:8080', enabled: true, secret: false },
          { name: 'token', value: '', enabled: true, secret: true },
        ],
      },
    ],
    items: [
      {
        kind: 'folder',
        id: 'f1',
        name: 'Auth',
        readOnly: false,
        variables: [],
        auth: { kind: 'inherit' },
        certificateCheck: 'inherit',
        items: [{ kind: 'request', id: 'r1', name: 'Login', method: 'POST', readOnly: false }],
      },
      { kind: 'request', id: 'r2', name: 'Health', method: 'GET', readOnly: false },
    ],
  };
}

export function draft(
  id: string,
  name: string,
  overrides: Partial<RequestDraft> = {},
): RequestDraft {
  return {
    id,
    name,
    method: 'GET',
    url: `https://example.com/${id}`,
    headers: [],
    body: { kind: 'none' },
    variables: [],
    auth: { kind: 'inherit' },
    certificateCheck: 'inherit',
    scripts: { preRequest: '', postResponse: '' },
    readOnly: false,
    ...overrides,
  };
}

export function changed(collection: CollectionView, createdId: string | null = null): CommandAck {
  return { type: 'collectionChanged', collection, createdId };
}

export class FakeApi implements Api {
  commands: Command[] = [];
  queries: Query[] = [];
  answers: Answer[] = [];
  queryAnswers: QueryAnswer[] = [];
  handler: ((event: AppEvent) => void) | null = null;
  unsubscribed = false;
  nextRun = 1;
  /** What the folder picker returns. */
  folder: string | null = null;
  /** Emitted before `execute` resolves, to simulate a very fast response. */
  emitBeforeAck: RunOutcome | null = null;

  async execute(command: Command): Promise<CommandAck> {
    this.commands.push(command);
    const answer = this.answers.shift();
    if (answer && 'fail' in answer) throw answer.fail;
    if (answer) return answer;
    if (command.type === 'sendRequest') {
      const runId = this.nextRun++;
      if (this.emitBeforeAck) this.emit(runId, this.emitBeforeAck);
      return { type: 'runStarted', runId };
    }
    throw new Error(`no answer queued for ${command.type}`);
  }

  async query(query: Query): Promise<QueryResult> {
    this.queries.push(query);
    const answer = this.queryAnswers.shift();
    if (answer && 'fail' in answer) throw answer.fail;
    if (answer) return answer;
    if (query.type === 'request') {
      return { type: 'request', request: draft(query.requestId, `Request ${query.requestId}`) };
    }
    throw new Error(`no answer queued for ${query.type}`);
  }

  async onEvent(handler: (event: AppEvent) => void) {
    this.handler = handler;
    return () => {
      this.unsubscribed = true;
    };
  }

  /** The options of every file and folder dialog opened. */
  picks: { kind: 'file' | 'folder'; options: PickOptions | undefined }[] = [];

  async pickFolder(options?: PickOptions): Promise<string | null> {
    this.picks.push({ kind: 'folder', options });
    return this.folder;
  }

  /** What the file picker returns. */
  file: string | null = null;

  async pickFile(options?: PickOptions): Promise<string | null> {
    this.picks.push({ kind: 'file', options });
    return this.file;
  }

  emit(runId: number, outcome: RunOutcome) {
    this.handler?.({ type: 'runFinished', runId, outcome });
  }
}

/** In-memory storage for the remembered collections. */
export function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, value),
    data,
  };
}
