// The script API as the app would describe it (sample for the spike).
interface ScriptRequest {
  method: string;
  url: string;
  setHeader(name: string, value: string): void;
  headers(): Record<string, string>;
}
interface ScriptResponse {
  status: number;
  text(): string;
  json<T = unknown>(): T;
  header(name: string): string | undefined;
}
interface ScriptEnvironment {
  get(name: string): string | undefined;
  set(name: string, value: string): void;
}
declare const req: ScriptRequest;
declare const res: ScriptResponse;
declare const env: ScriptEnvironment;
declare function log(...values: unknown[]): void;
declare function expect(value: unknown): { toBe(expected: unknown): void; toEqual(expected: unknown): void };
