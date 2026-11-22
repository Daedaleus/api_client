// Grouping and filtering the request history for the sidebar (pure, so it
// is tested on its own).

import type { HistoryItemView } from './types';

/** Entries of one day. */
export interface HistoryDay {
  /** Midnight of the day (local time), as milliseconds. */
  day: number;
  entries: HistoryItemView[];
}

/** Entries matching `filter` (method, URL, status or collection), grouped by local day, newest first. */
export function groupByDay(entries: HistoryItemView[], filter: string): HistoryDay[] {
  const wanted = filter.trim().toLowerCase();
  const days: HistoryDay[] = [];
  for (const entry of entries) {
    const text = [entry.method, entry.url, entry.status ?? '', entry.collection ?? '']
      .join(' ')
      .toLowerCase();
    if (wanted && !text.includes(wanted)) continue;
    const date = new Date(entry.atMs);
    const day = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const last = days.at(-1);
    if (last?.day === day) last.entries.push(entry);
    else days.push({ day, entries: [entry] });
  }
  return days;
}
