import type { AppEvent, Command, CommandAck, Query, QueryResult } from './types';

/**
 * Everything the UI needs from the backend. The Tauri implementation lives in
 * `tauri.ts`; tests use a fake. Components never call Tauri directly.
 */
export interface Api {
  /** Carries out a command; rejects with a `UserText` if it cannot. */
  execute(command: Command): Promise<CommandAck>;
  /** Answers a query; rejects with a `UserText` if it cannot. */
  query(query: Query): Promise<QueryResult>;
  /** Calls `handler` for every application event; resolves to an unsubscribe function. */
  onEvent(handler: (event: AppEvent) => void): Promise<() => void>;
}
