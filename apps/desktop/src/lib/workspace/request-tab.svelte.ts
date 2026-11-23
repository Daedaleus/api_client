// One editor tab: the request being edited, its saved state and its last
// run. Saved tabs belong to a request in a collection; a scratch tab is a
// quick request that is not saved anywhere.

import type { RequestDraft, RunOutcome, UserText } from '../types';

/** The request in a collection a tab edits. */
export interface SavedRequest {
  collectionId: string;
  requestId: string;
}

let nextKey = 1;

function snapshot(draft: RequestDraft): string {
  return JSON.stringify(draft);
}

export class RequestTab {
  readonly key = `tab-${nextKey++}`;
  readonly saved: SavedRequest | null;
  draft = $state<RequestDraft>(RequestTab.emptyDraft());
  /** Run whose result is awaited; `null` when idle. */
  pendingRun = $state<number | null>(null);
  outcome = $state<RunOutcome | null>(null);
  /**
   * Run the shown outcome belongs to (its body can be saved while the app
   * keeps it); `null` for one opened from the history.
   */
  outcomeRun = $state<number | null>(null);
  /** Why the last send or save failed. */
  error = $state<UserText | null>(null);
  /** Something to point out about the request (e.g. what an import left out). */
  notice = $state<UserText | null>(null);
  saving = $state(false);
  /** The draft as last saved, to tell whether there are unsaved changes. */
  #savedSnapshot = $state('');

  constructor(draft: RequestDraft, saved: SavedRequest | null) {
    this.draft = draft;
    this.saved = saved;
    this.#savedSnapshot = snapshot(draft);
  }

  static emptyDraft(): RequestDraft {
    return {
      id: '',
      name: '',
      method: 'GET',
      url: '',
      headers: [],
      pathParams: [],
      disabledQuery: [],
      body: { kind: 'none' },
      variables: [],
      auth: { kind: 'inherit' },
      certificateCheck: 'inherit',
      transport: { redirects: 'inherit', timeoutSeconds: null, cookies: 'inherit' },
      scripts: { preRequest: '', postResponse: '' },
      docs: '',
      readOnly: false,
    };
  }

  static scratch(): RequestTab {
    return new RequestTab(RequestTab.emptyDraft(), null);
  }

  get sending(): boolean {
    return this.pendingRun !== null;
  }

  /** Saved request with changes that are not on disk yet. */
  get dirty(): boolean {
    return this.saved !== null && snapshot(this.draft) !== this.#savedSnapshot;
  }

  isFor(collectionId: string, requestId?: string): boolean {
    return (
      this.saved?.collectionId === collectionId &&
      (requestId === undefined || this.saved.requestId === requestId)
    );
  }

  /** The current draft is what is on disk now. */
  markSaved(): void {
    this.#savedSnapshot = snapshot(this.draft);
  }

  /** The request was renamed elsewhere (tree): take the name, keep other edits. */
  renamed(name: string): void {
    const saved = JSON.parse(this.#savedSnapshot) as RequestDraft;
    this.draft.name = name;
    this.#savedSnapshot = snapshot({ ...saved, name });
  }
}
