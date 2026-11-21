import { describe, expect, it } from 'vitest';

import type { BodyDraft } from './types';
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

  it('keeps the fields between form and multipart, and the file of a binary body', () => {
    const form: BodyDraft = {
      kind: 'form',
      fields: [{ name: 'q', value: 'tea', enabled: true }],
    };
    const multipart = withKind(form, 'multipart');
    expect(multipart).toEqual({
      kind: 'multipart',
      fields: [{ name: 'q', value: 'tea', enabled: true, file: false }],
    });
    const withFile: BodyDraft = {
      kind: 'multipart',
      fields: [
        { name: 'q', value: 'tea', enabled: true, file: false },
        { name: 'avatar', value: 'a.png', enabled: true, file: true },
      ],
    };
    expect(withKind(withFile, 'multipart')).toEqual(withFile);
    expect(withKind(withFile, 'form')).toEqual(form);
    expect(withKind({ kind: 'text', content: 'x' }, 'form')).toEqual({ kind: 'form', fields: [] });
    expect(withKind(form, 'binary')).toEqual({ kind: 'binary', file: '' });
    const binary: BodyDraft = { kind: 'binary', file: 'data.bin' };
    expect(withKind(binary, 'binary')).toEqual(binary);
    expect(withKind(binary, 'json')).toEqual({ kind: 'json', content: '' });
    expect(bodyContent(binary)).toBe('');
  });
});
