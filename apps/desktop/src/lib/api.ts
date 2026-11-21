import type { AppEvent, Command, CommandAck, Query, QueryResult } from './types';

/**
 * Everything the UI needs from the backend and the host. The Tauri
 * implementation lives in `tauri.ts`; tests use a fake. Components never call
 * Tauri directly.
 */
export interface Api {
  /** Carries out a command; rejects with a `UserText` if it cannot. */
  execute(command: Command): Promise<CommandAck>;
  /** Answers a query; rejects with a `UserText` if it cannot. */
  query(query: Query): Promise<QueryResult>;
  /** Calls `handler` for every application event; resolves to an unsubscribe function. */
  onEvent(handler: (event: AppEvent) => void): Promise<() => void>;
  /** Lets the user pick a folder; resolves to its path, or `null` if cancelled. */
  pickFolder(): Promise<string | null>;
  /** Lets the user pick a file; resolves to its path, or `null` if cancelled. */
  pickFile(): Promise<string | null>;
}
