// Contract types with the Rust side, generated into bindings.ts by
// tauri-specta (`mise run bindings`). The UI only receives what Rust
// serialized, so it uses the `_Serialize` variants under shorter names.

import type {
  AppEvent_Serialize,
  ResponseView_Serialize,
  RunOutcomeView_Serialize,
  UserText,
} from './bindings';

export type {
  BodyView,
  Command,
  CommandAck,
  HeaderView,
  StatusClassView as StatusClass,
  UserText,
} from './bindings';

export type AppEvent = AppEvent_Serialize;
export type ResponseView = ResponseView_Serialize;
export type RunOutcome = RunOutcomeView_Serialize;

export function isUserText(value: unknown): value is UserText {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as UserText).key === 'string' &&
    typeof (value as UserText).args === 'object'
  );
}
