import { describe, expect, it } from 'vitest';

import { SECRET_MASK, hasPlaceholders, preview, variableValues } from './variables';

describe('variables', () => {
  it('takes enabled variables by trimmed name, later rows winning', () => {
    const values = variableValues([
      { name: ' host ', value: 'a', enabled: true, secret: false },
      { name: 'host', value: 'b', enabled: true, secret: false },
      { name: 'token', value: 'secret', enabled: false, secret: false },
    ]);
    expect([...values]).toEqual([['host', 'b']]);
  });

  it('fills known placeholders and lists unknown names once', () => {
    const values = new Map([['host', 'api.test']]);
    expect(preview('https://{{ host }}/{{v}}/{{v}}?q={{x}}', values)).toEqual({
      text: 'https://api.test/{{v}}/{{v}}?q={{x}}',
      missing: ['v', 'x'],
    });
  });

  it('does not resolve values again and keeps unclosed braces', () => {
    const values = new Map([['a', '{{b}}']]);
    expect(preview('{{a}} {{b', values)).toEqual({ text: '{{b}} {{b', missing: [] });
  });

  it('detects placeholders', () => {
    expect(hasPlaceholders('{{host}}/x')).toBe(true);
    expect(hasPlaceholders('{{host')).toBe(false);
    expect(hasPlaceholders('plain')).toBe(false);
  });

  it('never shows secret values, not even ones just typed', () => {
    const values = variableValues([{ name: 'token', value: 'typed', enabled: true, secret: true }]);
    expect(preview('?t={{token}}', values).text).toBe(`?t=${SECRET_MASK}`);
  });
});
