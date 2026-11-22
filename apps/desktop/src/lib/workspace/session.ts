// The open tabs, remembered between starts in the web view's storage:
// saved requests by id (with their draft if it has unsaved changes),
// scratch requests with their draft, and which tab was active. Credentials
// typed literally into a draft are not remembered – only `{{variables}}`.

import type { AuthDraft, HeaderDraft, RequestDraft } from '../types';
import type { RequestTab, SavedRequest } from './request-tab.svelte';

/** Version of the stored shape, so a later change can migrate or drop it. */
export const SESSION_VERSION = 1;

/** One remembered tab. */
export interface StoredTab {
  /** The saved request it edits; `null` for a scratch tab. */
  saved: SavedRequest | null;
  /** The draft, if it is a scratch tab or has unsaved changes. */
  draft: RequestDraft | null;
}

export interface StoredSession {
  version: number;
  tabs: StoredTab[];
  /** Index of the active tab. */
  active: number;
}

/** Header names whose values are credentials. */
const CREDENTIAL_HEADERS = ['authorization', 'proxy-authorization', 'cookie'];

/** `value` if it is only `{{variables}}` (and spaces), else empty. */
function keepTemplate(value: string): string {
  return /^(\s*\{\{[^{}]+\}\}\s*)*$/.test(value) ? value : '';
}

function withoutSecretAuth(auth: AuthDraft): AuthDraft {
  switch (auth.kind) {
    case 'basic':
      return { ...auth, password: keepTemplate(auth.password) };
    case 'bearer':
      return { ...auth, token: keepTemplate(auth.token) };
    case 'apiKey':
      return { ...auth, value: keepTemplate(auth.value) };
    case 'oauth2':
      return { ...auth, clientSecret: keepTemplate(auth.clientSecret) };
    default:
      return auth;
  }
}

/** `draft` without credentials typed literally (auth secrets, credential headers). */
export function withoutCredentials(draft: RequestDraft): RequestDraft {
  return {
    ...draft,
    auth: withoutSecretAuth(draft.auth),
    headers: draft.headers.map((header): HeaderDraft =>
      CREDENTIAL_HEADERS.includes(header.name.trim().toLowerCase())
        ? { ...header, value: keepTemplate(header.value) }
        : header,
    ),
  };
}

/** A scratch tab nobody typed into. */
function isEmptyScratch(tab: RequestTab): boolean {
  if (tab.saved) return false;
  const draft = tab.draft;
  return (
    draft.url === '' &&
    draft.headers.length === 0 &&
    draft.body.kind === 'none' &&
    draft.scripts.preRequest === '' &&
    draft.scripts.postResponse === ''
  );
}

/** What to remember of the open tabs. */
export function sessionOf(tabs: RequestTab[], activeKey: string | null): StoredSession {
  const kept = tabs.filter((tab) => !isEmptyScratch(tab));
  return {
    version: SESSION_VERSION,
    tabs: kept.map((tab) => ({
      saved: tab.saved ? { ...tab.saved } : null,
      draft:
        tab.saved && !tab.dirty
          ? null
          : withoutCredentials(JSON.parse(JSON.stringify(tab.draft)) as RequestDraft),
    })),
    active: Math.max(
      0,
      kept.findIndex((tab) => tab.key === activeKey),
    ),
  };
}

/** Whether `value` is a remembered session of this version. */
export function isSession(value: unknown): value is StoredSession {
  if (typeof value !== 'object' || value === null) return false;
  const session = value as Partial<StoredSession>;
  return (
    session.version === SESSION_VERSION &&
    typeof session.active === 'number' &&
    Array.isArray(session.tabs) &&
    session.tabs.every(
      (tab: unknown) =>
        typeof tab === 'object' &&
        tab !== null &&
        'saved' in tab &&
        'draft' in tab &&
        (tab.saved === null ||
          (typeof tab.saved === 'object' &&
            typeof (tab.saved as SavedRequest).collectionId === 'string' &&
            typeof (tab.saved as SavedRequest).requestId === 'string')),
    )
  );
}
