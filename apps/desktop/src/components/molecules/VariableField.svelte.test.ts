import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '../../lib/i18n/i18n.svelte';
import type { VariableInfo } from '../../lib/variables';
import VariableField from './VariableField.svelte';

const variables = new Map<string, VariableInfo>([
  ['baseUrl', { value: 'http://localhost:8080', source: 'environment' }],
  ['token', { value: '••••••', source: 'environment' }],
  ['userId', { value: '42', source: 'request' }],
]);

function setup(value = '') {
  const state = $state({ value });
  render(VariableField, {
    props: {
      get value() {
        return state.value;
      },
      set value(next: string) {
        state.value = next;
      },
      variables,
      label: 'URL',
    },
  });
  return { state, field: screen.getByLabelText('URL') as HTMLInputElement };
}

describe('VariableField', () => {
  beforeEach(() => i18n.setLocale('en'));

  it('suggests variables after {{ and completes them with the keyboard', async () => {
    const { state, field } = setup();
    await userEvent.type(field, '{{{{');
    expect(screen.getAllByRole('option').map((option) => option.textContent)).toEqual([
      expect.stringContaining('baseUrl'),
      expect.stringContaining('token'),
      expect.stringContaining('userId'),
    ]);
    expect(screen.getAllByRole('option')[0]).toHaveTextContent('environment');
    await userEvent.type(field, 'us');
    expect(screen.getAllByRole('option')).toHaveLength(1);
    await userEvent.keyboard('{Enter}');
    expect(state.value).toBe('{{userId}}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();

    // Arrows move, Escape closes, a pointer picks.
    await userEvent.type(field, '/{{{{');
    await userEvent.keyboard('{ArrowDown}{ArrowDown}{ArrowUp}{ArrowUp}{ArrowUp}{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    await userEvent.type(field, 'b');
    await userEvent.pointer({ keys: '[MouseLeft>]', target: screen.getByRole('option') });
    expect(state.value).toBe('{{userId}}/{{baseUrl}}');
  });

  it('explains its variables and marks unknown ones', () => {
    const { field } = setup('{{baseUrl}}/users/{{nope}}');
    expect(field.title).toBe('baseUrl = http://localhost:8080 (environment)\nnope: unknown');
    expect(field).toHaveClass('variable-field__input--unknown');
  });
});
