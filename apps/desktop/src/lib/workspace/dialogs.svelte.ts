// Questions to the user (a name, a confirmation) and notes to read, shown
// in the page's dialog. Callers simply await the answer.

import type { UserText } from '../types';

export type DialogRequest =
  | { kind: 'name'; title: UserText; value: string; answer(value: string | null): void }
  | { kind: 'confirm'; title: UserText; message: UserText; answer(ok: boolean): void }
  | {
      kind: 'info';
      title: UserText;
      message: UserText;
      items: UserText[];
      answer(ok: boolean): void;
    };

export class Dialogs {
  current = $state<DialogRequest | null>(null);

  /** Asks for a name; resolves to the trimmed name or `null` if cancelled or empty. */
  askName(title: UserText, value = ''): Promise<string | null> {
    return new Promise((resolve) => {
      this.current = {
        kind: 'name',
        title,
        value,
        answer: (name) => {
          this.current = null;
          const trimmed = name?.trim() ?? '';
          resolve(trimmed === '' ? null : trimmed);
        },
      };
    });
  }

  /** Shows `message` with a list of `items`; resolves when closed. */
  inform(title: UserText, message: UserText, items: UserText[]): Promise<void> {
    return new Promise((resolve) => {
      this.current = {
        kind: 'info',
        title,
        message,
        items,
        answer: () => {
          this.current = null;
          resolve();
        },
      };
    });
  }

  /** Asks for a confirmation; resolves to `true` if confirmed. */
  confirm(title: UserText, message: UserText): Promise<boolean> {
    return new Promise((resolve) => {
      this.current = {
        kind: 'confirm',
        title,
        message,
        answer: (ok) => {
          this.current = null;
          resolve(ok);
        },
      };
    });
  }
}
