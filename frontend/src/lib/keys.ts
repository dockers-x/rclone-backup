type KeyOptions = {
  /** Arrow keys wrap around the ends (tabs) instead of stopping (sliders). */
  wrap?: boolean;
  /** Up and Down also move, as on a slider. */
  vertical?: boolean;
  /** How far one arrow press moves. Default 1. */
  step?: number;
  /** How far PageUp and PageDown move; leave out to ignore them. */
  page?: number;
};

/**
 * The index a key press moves to in a row of `count` items, or null when the key does not move.
 * Shared by tab rows and chart scrubbers so they all answer the keyboard the same way.
 */
export function keyToIndex(
  key: string,
  index: number,
  count: number,
  { wrap = false, vertical = false, step = 1, page }: KeyOptions = {},
): number | null {
  if (count < 1) return null;
  const last = count - 1;
  const move = (by: number) =>
    wrap
      ? (((index + by) % count) + count) % count
      : Math.max(0, Math.min(last, index + by));
  switch (key) {
    case "ArrowRight":
      return move(step);
    case "ArrowLeft":
      return move(-step);
    case "ArrowUp":
      return vertical ? move(step) : null;
    case "ArrowDown":
      return vertical ? move(-step) : null;
    case "PageUp":
      return page ? move(page) : null;
    case "PageDown":
      return page ? move(-page) : null;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}
