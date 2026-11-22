import '@testing-library/jest-dom/vitest';

// jsdom does no layout; CodeMirror measures text ranges. Empty measurements
// are enough for tests of what it shows and does.
const noRects = (): DOMRectList => {
  const list: DOMRect[] = [];
  return Object.assign(list, { item: () => null }) as unknown as DOMRectList;
};
Range.prototype.getClientRects = noRects;
Range.prototype.getBoundingClientRect = () => new DOMRect();
