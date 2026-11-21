import type { BodyDraft, FormFieldDraft, MultipartFieldDraft } from './types';

export type BodyKind = BodyDraft['kind'];

/** Body kinds offered in the editor, in display order. */
export const BODY_KINDS: readonly BodyKind[] = [
  'none',
  'json',
  'text',
  'form',
  'multipart',
  'binary',
];

/** The text of a JSON or text body (empty for every other kind). */
export function bodyContent(body: BodyDraft): string {
  return body.kind === 'json' || body.kind === 'text' ? body.content : '';
}

/** The text fields of a form or multipart body. */
function textFields(body: BodyDraft): FormFieldDraft[] {
  if (body.kind === 'form') return body.fields;
  if (body.kind === 'multipart') {
    return body.fields
      .filter((field) => !field.file)
      .map(({ name, value, enabled }) => ({ name, value, enabled }));
  }
  return [];
}

/**
 * A body of `kind`, keeping what fits: the text between JSON and text, the
 * fields between form and multipart.
 */
export function withKind(body: BodyDraft, kind: BodyKind): BodyDraft {
  switch (kind) {
    case 'none':
      return { kind };
    case 'json':
    case 'text':
      return { kind, content: bodyContent(body) };
    case 'form':
      return { kind, fields: textFields(body) };
    case 'multipart': {
      const fields: MultipartFieldDraft[] =
        body.kind === 'multipart'
          ? body.fields
          : textFields(body).map((field) => ({ ...field, file: false }));
      return { kind, fields };
    }
    case 'binary':
      return { kind, file: body.kind === 'binary' ? body.file : '' };
  }
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
