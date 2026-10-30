import { LiquidBar, type LiquidLook } from "../parts";
import type { LiquidTabsWorldProps } from "../liquid-tabs";

const look: LiquidLook = {
  world: "craft",
  track: {
    background: "var(--ovio-track)",
    borderRadius: 18,
    boxShadow:
      "inset 0 2px 4px rgba(70,45,20,.15), 0 1px 0 rgba(255,255,255,.8)",
  },
  blob: "var(--ovio-accent)",
  blobRadius: "999px",
  on: "var(--on-accent)",
  off: "var(--muted)",
  tab: "py-3.25 text-[11px] font-bold sm:text-[15px] tracking-[-0.01em]",
  label: "text-[13px]",
  statusText: "text-xl font-bold",
  // Paper settling: the edges overshoot a touch and wobble into place.
  liquid: {
    lead: { stiffness: 420, damping: 28 },
    trail: { stiffness: 200, damping: 20 },
    squash: 0.08,
    round: true,
  },
  dots: ["#c9b9a0", "#ee9f63", "#2f9a45"],
};

export default function CraftLiquidTabs(props: LiquidTabsWorldProps) {
  return <LiquidBar look={look} {...props} />;
}
