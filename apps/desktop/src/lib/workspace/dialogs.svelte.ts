// Questions to the user (a name, a confirmation) and notes to read, shown
// in the page's dialog. Callers simply await the answer.

import type { UserText } from '../types';

export type DialogRequest =
  | {
      kind: 'name';
      title: UserText;
      /** The field's label (`dialog.name` for names). */
      label: UserText;
      value: string;
      answer(value: string | null): void;
    }
  | { kind: 'confirm'; title: UserText; message: UserText; answer(ok: boolean): void }
  | {
      kind: 'choose';
      title: UserText;
      /** The list's label. */
      label: UserText;
      /** Values with their display texts (already in the user's language or data). */
      options: { value: string; label: string }[];
      value: string;
      answer(value: string | null): void;
    }
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
    return this.askText(title, { key: 'dialog.name', args: {} }, value);
  }

  /** Asks for a line of text with `label`; the trimmed text or `null`. */
  askText(title: UserText, label: UserText, value = ''): Promise<string | null> {
    return new Promise((resolve) => {
      this.current = {
        kind: 'name',
        title,
        label,
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

  /** Asks to pick one of `options`; resolves to its value or `null` if cancelled. */
  choose(
    title: UserText,
    label: UserText,
    options: { value: string; label: string }[],
  ): Promise<string | null> {
    return new Promise((resolve) => {
      this.current = {
        kind: 'choose',
        title,
        label,
        options,
        value: options[0]?.value ?? '',
        answer: (value) => {
          this.current = null;
          resolve(value);
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
