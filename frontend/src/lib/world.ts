export const WORLDS = ["minimal", "craft", "retro", "toy"] as const;

export type World = (typeof WORLDS)[number];

/** A component's world implementations. A single-world install from the registry has just one. */
export type WorldViews<V> = Partial<Record<World, V>>;

/**
 * The worlds a component's worlds/index.ts exports, for its `variant` prop: asking a single-world
 * install for another world is a type error rather than a silent fallback.
 */
export type InstalledWorld<Views> = keyof Views & World;

/**
 * Every world mapped to an installed view: its own, else Minimal, else whichever world is
 * installed. A single-world install then draws that world whatever a provider asks for.
 */
export function everyWorld<V>(views: WorldViews<V>): Record<World, V> {
  const fallback =
    views.minimal ?? WORLDS.map((w) => views[w]).find((v) => v !== undefined);
  if (fallback === undefined)
    throw new Error("No Ovio world is installed for this component");
  return Object.fromEntries(
    WORLDS.map((w) => [w, views[w] ?? fallback]),
  ) as Record<World, V>;
}
