import type { BodyDraft } from './types';

export type BodyKind = BodyDraft['kind'];

/** Body kinds offered in the editor, in display order. */
export const BODY_KINDS: readonly BodyKind[] = ['none', 'json', 'text'];

/** The text of a body (empty for no body). */
export function bodyContent(body: BodyDraft): string {
  return body.kind === 'none' ? '' : body.content;
}

/** A body of `kind`; the text is kept when switching between JSON and text. */
export function withKind(body: BodyDraft, kind: BodyKind): BodyDraft {
  return kind === 'none' ? { kind } : { kind, content: bodyContent(body) };
}

/** Why `text` is not valid JSON, or `null` if it is (or is empty). */
export function jsonProblem(text: string): string | null {
  if (text.trim() === '') return null;
  try {
    JSON.parse(text);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}
