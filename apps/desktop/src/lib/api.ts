import type {
  AppEvent,
  AssistRequest,
  AssistResult,
  Command,
  CommandAck,
  Query,
  QueryResult,
} from './types';

/** Files a file dialog offers: a name and extensions (`*` for all). */
export interface FileFilter {
  name: string;
  extensions: string[];
}

/** Title and filters of a file or folder dialog (texts already translated). */
export interface PickOptions {
  title?: string;
  filters?: FileFilter[];
}

/** A save dialog: like a pick, with a suggested file name. */
export interface SaveOptions extends PickOptions {
  defaultPath?: string;
}

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
  /** Downloads and imports a collection from `url`; rejects with a `UserText` if it cannot. */
  importUrl(url: string): Promise<CommandAck>;
  /** Checks or completes a script for the editor; rejects with a `UserText` if it cannot. */
  assist(request: AssistRequest): Promise<AssistResult>;
  /** Lets the user pick a folder; resolves to its path, or `null` if cancelled. */
  pickFolder(options?: PickOptions): Promise<string | null>;
  /** Lets the user pick a file; resolves to its path, or `null` if cancelled. */
  pickFile(options?: PickOptions): Promise<string | null>;
  /** Lets the user choose where to save a file; resolves to its path, or `null` if cancelled. */
  pickSaveFile(options?: SaveOptions): Promise<string | null>;
}
