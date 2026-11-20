import { describe, expect, it } from 'vitest';

import { bodyContent, jsonProblem, withKind } from './body';

describe('body helpers', () => {
  it('keeps the text when switching between JSON and text', () => {
    const json = { kind: 'json', content: '{"a":1}' } as const;
    expect(withKind(json, 'text')).toEqual({ kind: 'text', content: '{"a":1}' });
    expect(withKind(json, 'none')).toEqual({ kind: 'none' });
    expect(withKind({ kind: 'none' }, 'json')).toEqual({ kind: 'json', content: '' });
    expect(bodyContent({ kind: 'none' })).toBe('');
  });

  it('explains invalid JSON and accepts valid or empty text', () => {
    expect(jsonProblem('{"price": 1.10}')).toBeNull();
    expect(jsonProblem('   ')).toBeNull();
    expect(jsonProblem('{oops')).toMatch(/JSON|property|token/i);
  });
});
