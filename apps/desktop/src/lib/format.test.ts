import { describe, expect, it } from 'vitest';

import { formatBytes, formatDuration } from './format';

describe('formatBytes', () => {
  it('uses the largest fitting unit', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1023)).toBe('1023 B');
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
    expect(formatBytes(3 * 1024 ** 4)).toBe('3072.0 GB');
  });
});

describe('formatDuration', () => {
  it('shows milliseconds below a second', () => {
    expect(formatDuration(0)).toBe('0 ms');
    expect(formatDuration(999)).toBe('999 ms');
  });

  it('shows seconds from one second on', () => {
    expect(formatDuration(1000)).toBe('1.00 s');
    expect(formatDuration(1250)).toBe('1.25 s');
  });
});
