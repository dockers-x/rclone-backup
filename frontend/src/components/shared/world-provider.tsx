"use client";

import { createContext, useContext, type ReactNode } from "react";
import type { World } from "@/lib/world";

const WorldContext = createContext<World | null>(null);

/** Sets the default world for every Ovio component below it. A component's `variant` prop wins. */
export function OvioProvider({
  world,
  children,
}: {
  world: World;
  children: ReactNode;
}) {
  return (
    <WorldContext.Provider value={world}>{children}</WorldContext.Provider>
  );
}

/** Resolves the world for a component: its own `variant`, then the nearest provider, then Minimal. */
export function useWorld(variant?: World): World {
  const fromProvider = useContext(WorldContext);
  return variant ?? fromProvider ?? "minimal";
}

export type { World };
