import { describe, expect, it } from 'vitest';

import {
  complete,
  placeholdersIn,
  scopeInfo,
  suggestionAt,
  suggestionsFor,
  type VariableInfo,
} from './variables';

const row = (name: string, value: string, secret = false) => ({
  name,
  value,
  enabled: true,
  secret,
});

describe('variable suggestions', () => {
  it('knows where each value comes from, the stronger place winning', () => {
    const info = scopeInfo([
      { source: 'collection', variables: [row('baseUrl', 'http://c'), row('a', '1')] },
      { source: 'environment', variables: [row('baseUrl', 'http://dev'), row('token', 't', true)] },
      { source: 'request', variables: [row('', 'ignored')] },
    ]);
    expect(info.get('baseUrl')).toEqual({ value: 'http://dev', source: 'environment' });
    expect(info.get('a')).toEqual({ value: '1', source: 'collection' });
    expect(info.get('token')?.value).toBe('••••••');
    expect(info.has('')).toBe(false);
  });

  it('finds an open placeholder at the caret', () => {
    expect(suggestionAt('{{base', 6)).toEqual({ start: 2, typed: 'base' });
    expect(suggestionAt('x/{{ to', 7)).toEqual({ start: 4, typed: 'to' });
    expect(suggestionAt('{{a}}/b', 7)).toBeNull();
    expect(suggestionAt('plain', 5)).toBeNull();
    expect(suggestionAt('{{a}}/{{', 8)).toEqual({ start: 8, typed: '' });
  });

  it('suggests names starting with the typed text first', () => {
    const variables = new Map<string, VariableInfo>([
      ['token', { value: 't', source: 'environment' }],
      ['baseUrl', { value: 'u', source: 'collection' }],
      ['authToken', { value: 'a', source: 'request' }],
    ]);
    expect(suggestionsFor(variables, 'tok')).toEqual(['token', 'authToken']);
    expect(suggestionsFor(variables, '')).toEqual(['authToken', 'baseUrl', 'token']);
    expect(suggestionsFor(variables, 'zzz')).toEqual([]);
  });

  it('completes the placeholder and closes it once', () => {
    expect(complete('{{ba/x', 4, { start: 2, typed: 'ba' }, 'baseUrl')).toEqual({
      text: '{{baseUrl}}/x',
      caret: 11,
    });
    expect(complete('{{ba}}/x', 4, { start: 2, typed: 'ba' }, 'baseUrl')).toEqual({
      text: '{{baseUrl}}/x',
      caret: 11,
    });
  });

  it('explains the placeholders of a text', () => {
    const variables = new Map<string, VariableInfo>([
      ['baseUrl', { value: 'http://dev', source: 'environment' }],
    ]);
    expect(placeholdersIn('{{baseUrl}}/{{ id }}/{{baseUrl}}', variables)).toEqual([
      { name: 'baseUrl', info: { value: 'http://dev', source: 'environment' } },
      { name: 'id', info: null },
    ]);
  });
});
