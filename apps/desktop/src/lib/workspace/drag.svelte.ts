// Dragging requests and folders in the sidebar tree, with pointer events
// (HTML drag and drop is unreliable in the desktop web view). A press turns
// into a drag once the pointer moved a few pixels; the row under the pointer
// says where the item would land.

import type { TreeItemView } from '../types';
import type { DropPosition } from './tree';

/** Pixels the pointer has to move before a press becomes a drag. */
const DRAG_THRESHOLD = 5;

export interface DragSource {
  collectionId: string;
  item: TreeItemView;
}

export interface DropTarget {
  collectionId: string;
  item: TreeItemView;
  position: DropPosition;
}

export class TreeDrag {
  /** The item being dragged, once the pointer moved far enough. */
  source = $state<DragSource | null>(null);
  /** The row under the pointer and where on it. */
  target = $state<DropTarget | null>(null);
  #pressed: (DragSource & { x: number; y: number }) | null = null;
  /** A drag just ended: the click that follows must not open the item. */
  #justDropped = false;

  press(collectionId: string, item: TreeItemView, x: number, y: number): void {
    this.#pressed = { collectionId, item, x, y };
  }

  /** The pointer moved (anywhere in the window). */
  move(x: number, y: number): void {
    const pressed = this.#pressed;
    if (!pressed || this.source) return;
    if (Math.hypot(x - pressed.x, y - pressed.y) >= DRAG_THRESHOLD) {
      this.source = { collectionId: pressed.collectionId, item: pressed.item };
    }
  }

  /** The pointer is over a row while dragging. */
  hover(target: DropTarget): void {
    if (this.source) this.target = target;
  }

  /** The pointer was released: what was dropped where (`null` if it was no drag). */
  release(): { source: DragSource; target: DropTarget } | null {
    const { source, target } = this;
    this.#pressed = null;
    this.source = null;
    this.target = null;
    if (!source) return null;
    this.#justDropped = true;
    // The click follows in the same task as the release, after microtasks.
    setTimeout(() => (this.#justDropped = false), 0);
    if (!target || target.collectionId !== source.collectionId) return null;
    return { source, target };
  }

  /** Whether a click right now is the end of a drag (and should be ignored). */
  get consumesClick(): boolean {
    return this.#justDropped;
  }

  cancel(): void {
    this.#pressed = null;
    this.source = null;
    this.target = null;
  }
}
