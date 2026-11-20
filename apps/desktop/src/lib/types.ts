// Contract types with the Rust side, generated into bindings.ts by
// tauri-specta (`mise run bindings`). The UI receives what Rust serialized
// (`_Serialize` variants) and sends what Rust deserializes (`_Deserialize`);
// both are re-exported here under shorter names.

import type {
  AppEvent_Serialize,
  CollectionView,
  CommandAck_Serialize,
  Command_Deserialize,
  QueryResult_Serialize,
  RequestDraft_Deserialize,
  RequestDraft_Serialize,
  ResponseView_Serialize,
  RunOutcomeView_Serialize,
  UserText,
} from './bindings';

export type {
  BodyDraft,
  BodyView,
  CollectionView,
  HeaderDraft,
  HeaderView,
  Query,
  StatusClassView as StatusClass,
  TreeItemView,
  UserText,
} from './bindings';

export type AppEvent = AppEvent_Serialize;
export type Command = Command_Deserialize;
export type CommandAck = CommandAck_Serialize;
export type QueryResult = QueryResult_Serialize;
/** A saved request as the editor receives it. */
export type RequestDraft = RequestDraft_Serialize;
/** A saved request as the editor sends it back. */
export type RequestDraftInput = RequestDraft_Deserialize;
export type ResponseView = ResponseView_Serialize;
export type RunOutcome = RunOutcomeView_Serialize;

/** The collection in a `collectionChanged` acknowledgement. */
export type ChangedCollection = { collection: CollectionView; createdId: string | null };

export function isUserText(value: unknown): value is UserText {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as UserText).key === 'string' &&
    typeof (value as UserText).args === 'object'
  );
}
