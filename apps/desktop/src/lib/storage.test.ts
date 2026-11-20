import { describe, expect, it } from 'vitest';

import { memoryStorage } from '../test/fake-api';
import { readJson, writeJson } from './storage';

const isNumber = (value: unknown): value is number => typeof value === 'number';

const broken = {
  getItem: (): string | null => {
    throw new Error('disabled');
  },
  setItem: (): void => {
    throw new Error('disabled');
  },
};

describe('storage', () => {
  it('round-trips JSON values', () => {
    const storage = memoryStorage();
    writeJson(storage, 'n', 42);
    expect(readJson(storage, 'n', 0, isNumber)).toBe(42);
  });

  it('falls back for missing, invalid or unreadable values', () => {
    expect(readJson(memoryStorage(), 'n', 1, isNumber)).toBe(1);
    expect(readJson(memoryStorage({ n: '"text"' }), 'n', 1, isNumber)).toBe(1);
    expect(readJson(memoryStorage({ n: '{' }), 'n', 1, isNumber)).toBe(1);
    expect(readJson(broken, 'n', 1, isNumber)).toBe(1);
    expect(readJson(null, 'n', 1, isNumber)).toBe(1);
  });

  it('ignores failing writes', () => {
    expect(() => writeJson(broken, 'n', 1)).not.toThrow();
  });
});
