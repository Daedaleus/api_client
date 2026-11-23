// Docs of requests, folders and collections as HTML. Docs come with
// imported collections, so they are untrusted: the HTML is sanitized
// (no scripts, event handlers or `javascript:` links), and nothing is
// loaded while reading – no images, frames or styles that would fetch from
// a server (the user decides what is sent). Links are shown but do not
// navigate the app window (see DocsEditor).

import DOMPurify from 'dompurify';
import { marked } from 'marked';

/** Tags that would load something or do not belong in docs. */
const FORBIDDEN_TAGS = [
  'img',
  'picture',
  'source',
  'video',
  'audio',
  'iframe',
  'object',
  'embed',
  'style',
  'link',
  'form',
  'input',
  'button',
  'svg',
  'math',
];

/** Attributes that would load something or style the page. */
const FORBIDDEN_ATTRIBUTES = ['style', 'srcset', 'src', 'background', 'poster'];

// Links: their address as tooltip, so it can be read before copying it.
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A' && node.hasAttribute('href')) {
    node.setAttribute('title', node.getAttribute('href') ?? '');
  }
});

/** `text` (Markdown, GitHub flavour) as safe HTML. */
export function renderMarkdown(text: string): string {
  const html = marked.parse(text, { async: false, gfm: true, breaks: false });
  return DOMPurify.sanitize(html, {
    FORBID_TAGS: FORBIDDEN_TAGS,
    FORBID_ATTR: FORBIDDEN_ATTRIBUTES,
  });
}
