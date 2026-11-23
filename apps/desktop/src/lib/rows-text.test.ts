import { describe, expect, it } from 'vitest';

import { rowsToText, textToRows } from './rows-text';

describe('rows as text', () => {
  it('writes one row per line, switched-off ones commented out', () => {
    expect(
      rowsToText([
        { name: 'Accept', value: 'application/json', enabled: true },
        { name: 'X-Old', value: '1', enabled: false },
        { name: ' ', value: '', enabled: true },
        { name: 'Empty', value: '', enabled: true },
      ]),
    ).toBe('Accept: application/json\n// X-Old: 1\nEmpty: ');
    expect(rowsToText([])).toBe('');
  });

  it('reads pasted headers and parameters', () => {
    expect(
      textToRows(
        [
          'Accept: application/json',
          '  cookie:a=b; c=d  ',
          '',
          '//X-Old: 1',
          '// page=2',
          'next=http://x.test/?a=b',
          'X-Url: http://a.test:8080/',
          'flag',
          '//',
          'Authorization: Bearer {{token}}',
        ].join('\r\n'),
      ),
    ).toEqual([
      { name: 'Accept', value: 'application/json', enabled: true },
      { name: 'cookie', value: 'a=b; c=d', enabled: true },
      { name: 'X-Old', value: '1', enabled: false },
      { name: 'page', value: '2', enabled: false },
      { name: 'next', value: 'http://x.test/?a=b', enabled: true },
      { name: 'X-Url', value: 'http://a.test:8080/', enabled: true },
      { name: 'flag', value: '', enabled: true },
      { name: 'Authorization', value: 'Bearer {{token}}', enabled: true },
    ]);
  });

  it('round-trips', () => {
    const rows = [
      { name: 'Accept', value: 'text/html', enabled: true },
      { name: 'X-Off', value: 'a: b', enabled: false },
    ];
    expect(textToRows(rowsToText(rows))).toEqual(rows);
  });
});
