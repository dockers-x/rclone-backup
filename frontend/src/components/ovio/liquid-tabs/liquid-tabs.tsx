"use client";

import {
  type ComponentType,
  useCallback,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { useWorld } from "@/components/shared/world-provider";
import { everyWorld, type InstalledWorld } from "@/lib/world";
import { keyToIndex } from "@/lib/keys";
import * as worlds from "./worlds";

export type DeployStatus = "offline" | "building" | "online";

export interface LiquidTabsProps {
  tabs: string[];
  variant?: InstalledWorld<typeof worlds>;
  value?: number;
  defaultValue?: number;
  onValueChange?: (index: number) => void;
  /** Deploy indicator state. Leave out to hide the indicator. */
  status?: DeployStatus;
  /** Blur strength of the deploy status goo. */
  goo?: number;
  /** Content for each tab's panel, by index. */
  panels?: ReactNode[];
  /** Accessible name for the tab list. */
  label?: string;
  className?: string;
}

export type LiquidTabsWorldProps = {
  tabs: string[];
  index: number;
  select: (index: number) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  tabRef: (index: number) => (el: HTMLButtonElement | null) => void;
  tabId: (index: number) => string;
  panelId: string;
  /** A filter id unique to this instance, so several tab bars can share a page. */
  filterId: string;
  goo: number;
  status?: DeployStatus;
  statusLabel?: string;
  panel?: ReactNode;
  label: string;
  className?: string;
};

const STATUS_LABEL: Record<DeployStatus, string> = {
  offline: "Offline",
  building: "Building…",
  online: "Online",
};

export const STATUS_INDEX: Record<DeployStatus, number> = {
  offline: 0,
  building: 1,
  online: 2,
};

/** The installed worlds; a single-world install draws its one world for any provider world. */
const VIEWS = everyWorld<ComponentType<LiquidTabsWorldProps>>(worlds);

export function LiquidTabs({
  tabs,
  variant,
  value,
  defaultValue = 0,
  onValueChange,
  status,
  goo = 9,
  panels,
  label = "Sections",
  className,
}: LiquidTabsProps) {
  const world = useWorld(variant);
  const [inner, setInner] = useState(defaultValue);
  const index = Math.max(0, Math.min(tabs.length - 1, value ?? inner));
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const id = useId().replace(/[^a-zA-Z0-9_-]/g, "");

  const select = useCallback(
    (next: number) => {
      const clamped = Math.max(0, Math.min(tabs.length - 1, next));
      if (value === undefined) setInner(clamped);
      if (clamped !== index) onValueChange?.(clamped);
    },
    [index, onValueChange, tabs.length, value],
  );

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const next = keyToIndex(event.key, index, tabs.length);
    if (next === null) return;
    event.preventDefault();
    select(next);
    refs.current[next]?.focus();
  };

  const props: LiquidTabsWorldProps = {
    tabs,
    index,
    select,
    onKeyDown,
    tabRef: (i) => (el) => {
      refs.current[i] = el;
    },
    tabId: (i) => `${id}-tab-${i}`,
    panelId: `${id}-panel`,
    filterId: `ovio-goo-${id}`,
    goo,
    status,
    statusLabel: status ? STATUS_LABEL[status] : undefined,
    panel: panels?.[index],
    label,
    className,
  };

  // No tabs, nothing to show: an empty bar would size its pill as 100% / 0.
  if (tabs.length === 0) return null;
  const View = VIEWS[world] ?? VIEWS.minimal;
  return <View {...props} />;
}
