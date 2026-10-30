import { LiquidBar, type LiquidLook } from "../parts";
import type { LiquidTabsWorldProps } from "../liquid-tabs";

const look: LiquidLook = {
  world: "retro",
  track: {
    background: "var(--ovio-track)",
    border: "3px double #3fae55",
    borderRadius: 0,
  },
  blob: "var(--ovio-accent)",
  blobRadius: "0px",
  on: "var(--on-accent)",
  off: "var(--muted)",
  tab: "py-2 text-sm sm:text-2xl tracking-[0.04em] uppercase text-shadow-(--ovio-glow)",
  label: "text-lg tracking-[0.04em] uppercase",
  statusText: "text-[26px] uppercase text-shadow-(--ovio-glow)",
  // Old hardware: square ends, 12 frames a second, every edge on a 4px grid.
  liquid: {
    lead: { stiffness: 700, damping: 53 },
    trail: { stiffness: 300, damping: 35 },
    squash: 0,
    round: false,
    fps: 12,
    snap: 4,
  },
  dots: ["#1d5a2b", "#ffd34d", "#4fdc68"],
};

export default function RetroLiquidTabs(props: LiquidTabsWorldProps) {
  return <LiquidBar look={look} {...props} />;
}
