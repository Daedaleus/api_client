import { describe, expect, it } from 'vitest';

import { renderMarkdown } from './markdown';

describe('renderMarkdown', () => {
  it('renders Markdown', () => {
    const html = renderMarkdown('# Login\n\nNeeds `{{token}}` and **care**.\n\n- a\n- b');
    expect(html).toContain('<h1>Login</h1>');
    expect(html).toContain('<code>{{token}}</code>');
    expect(html).toContain('<strong>care</strong>');
    expect(html).toContain('<li>a</li>');
    expect(renderMarkdown('| a | b |\n|---|---|\n| 1 | 2 |')).toContain('<table>');
  });

  it('removes scripts, handlers and dangerous links', () => {
    const html = renderMarkdown(
      '<script>alert(1)</script>\n\n<b onclick="alert(2)">x</b>\n\n[evil](javascript:alert(3))',
    );
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onclick');
    expect(html).not.toContain('javascript:');
    expect(html).toContain('<b>x</b>');
  });

  it('loads nothing while reading', () => {
    const html = renderMarkdown(
      '![tracker](https://example.com/pixel.png)\n\n<iframe src="https://example.com"></iframe>\n\n<p style="background:url(https://x)">s</p>',
    );
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<iframe');
    expect(html).not.toContain('https://example.com/pixel.png');
    expect(html).not.toContain('style=');
    // Links stay, as text the user can read and copy.
    const link = renderMarkdown('[docs](https://example.com/docs)');
    expect(link).toContain('href="https://example.com/docs"');
    expect(link).toContain('title="https://example.com/docs"');
  });
});
