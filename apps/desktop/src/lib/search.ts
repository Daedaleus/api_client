// Finding requests in the open collections: the quick search (Ctrl+K) and
// the filter above the sidebar tree. Every word of the query must appear –
// in the request's name, method or URL, or in the names of its folders or
// collection – ignoring case.

import type { CollectionView, TreeItemView } from './types';

type RequestView = Extract<TreeItemView, { kind: 'request' }>;

/** A request found by the quick search. */
export interface SearchHit {
  collectionId: string;
  request: RequestView;
  /** Collection and folders around it, e.g. `Shop / Auth`. */
  path: string;
}

/** The lower-case words of `query`. */
function words(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}

function matchesAll(text: string, wanted: string[]): boolean {
  const haystack = text.toLowerCase();
  return wanted.every((word) => haystack.includes(word));
}

/** How well a hit fits: the exact name first, then names starting with the query, then containing it. */
function rank(hit: SearchHit, query: string): number {
  const name = hit.request.name.toLowerCase();
  if (name === query) return 0;
  if (name.startsWith(query)) return 1;
  if (name.includes(query)) return 2;
  return 3;
}

/** Requests matching `query` in all `collections`, best first, at most `limit`. */
export function searchRequests(
  collections: CollectionView[],
  query: string,
  limit = 50,
): SearchHit[] {
  const wanted = words(query);
  if (wanted.length === 0) return [];
  const hits: SearchHit[] = [];
  const walk = (collectionId: string, items: TreeItemView[], path: string) => {
    for (const item of items) {
      if (item.kind === 'folder') {
        walk(collectionId, item.items, `${path} / ${item.name}`);
      } else if (matchesAll(`${path} ${item.name} ${item.method} ${item.url}`, wanted)) {
        hits.push({ collectionId, request: item, path });
      }
    }
  };
  for (const collection of collections) walk(collection.id, collection.items, collection.name);
  const normalized = query.trim().toLowerCase();
  return hits
    .map((hit, index) => ({ hit, index, rank: rank(hit, normalized) }))
    .sort((a, b) => a.rank - b.rank || a.index - b.index)
    .slice(0, limit)
    .map(({ hit }) => hit);
}

/**
 * The tree with only what matches `query`: matching requests, folders whose
 * name matches (with everything in them) and folders containing matches.
 * An empty query keeps everything.
 */
export function filterTree(items: TreeItemView[], query: string): TreeItemView[] {
  const wanted = words(query);
  if (wanted.length === 0) return items;
  const keep = (list: TreeItemView[]): TreeItemView[] =>
    list.flatMap((item): TreeItemView[] => {
      if (item.kind === 'request') {
        return matchesAll(`${item.name} ${item.method} ${item.url}`, wanted) ? [item] : [];
      }
      if (matchesAll(item.name, wanted)) return [item];
      const children = keep(item.items);
      return children.length > 0 ? [{ ...item, items: children }] : [];
    });
  return keep(items);
}
