import { type UserText, isUserText } from './types';

/** Turns anything a failed command or query rejects with into a display text. */
export function toUserText(error: unknown): UserText {
  if (isUserText(error)) {
    return error;
  }
  const detail = error instanceof Error ? error.message : String(error);
  return { key: 'error.unexpected', args: { detail } };
}
