// Contract types with the Rust side, generated into bindings.ts by
// tauri-specta (`mise run bindings`). The UI receives what Rust serialized
// (`_Serialize` variants) and sends what Rust deserializes (`_Deserialize`);
// both are re-exported here under shorter names.

import type {
  AppEvent_Serialize,
  CollectionView_Serialize,
  CommandAck_Serialize,
  Command_Deserialize,
  EnvironmentDraft_Deserialize,
  EnvironmentDraft_Serialize,
  HistoryItemView_Serialize,
  QueryResult_Serialize,
  RequestDraft_Deserialize,
  RequestDraft_Serialize,
  ResponseView_Serialize,
  RunOutcomeView_Serialize,
  ScriptsDraft_Serialize,
  TreeItemView_Serialize,
  UserText,
  VariableDraft_Serialize,
} from './bindings';

export type {
  AssistRequest,
  AssistResult,
  AuthDraft,
  BodyDraft,
  BodyView,
  CompletionView,
  DiagnosticView,
  CertificateCheckView,
  FormFieldDraft,
  MultipartFieldDraft,
  SendContext,
  HeaderDraft,
  HeaderView,
  OutgoingRequest,
  RedirectsView,
  Query,
  RecipientsView,
  StatusClassView as StatusClass,
  TestResultView,
  TransportDraft,
  UserText,
} from './bindings';

export type AppEvent = AppEvent_Serialize;
export type Command = Command_Deserialize;
export type CommandAck = CommandAck_Serialize;
export type CollectionView = CollectionView_Serialize;
export type TreeItemView = TreeItemView_Serialize;
/** An environment as the app reports it. */
export type EnvironmentDraft = EnvironmentDraft_Serialize;
/** An environment as the editor sends it back. */
export type EnvironmentDraftInput = EnvironmentDraft_Deserialize;
/** A variable as the app reports it (secret values always empty). */
export type VariableDraft = VariableDraft_Serialize;
export type QueryResult = QueryResult_Serialize;
/** A saved request as the editor receives it. */
export type RequestDraft = RequestDraft_Serialize;
/** A saved request as the editor sends it back. */
export type RequestDraftInput = RequestDraft_Deserialize;
export type ResponseView = ResponseView_Serialize;
export type RunOutcome = RunOutcomeView_Serialize;
export type ScriptsDraft = ScriptsDraft_Serialize;
export type HistoryItemView = HistoryItemView_Serialize;

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
