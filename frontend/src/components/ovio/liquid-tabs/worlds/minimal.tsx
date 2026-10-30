import { LiquidBar, type LiquidLook } from "../parts";
import type { LiquidTabsWorldProps } from "../liquid-tabs";

const look: LiquidLook = {
  world: "minimal",
  track: { background: "var(--ovio-track)", borderRadius: 14 },
  blob: "var(--ovio-accent)",
  blobRadius: "999px",
  on: "var(--on-accent)",
  off: "var(--muted)",
  tab: "py-3 text-[11px] font-medium sm:text-sm tracking-[-0.01em]",
  label: "text-xs",
  statusText: "text-lg font-medium",
  // A clean glide: both edges critically damped, so it stretches and gathers without overshoot.
  liquid: {
    lead: { stiffness: 520, damping: 46 },
    trail: { stiffness: 230, damping: 30 },
    squash: 0.06,
    round: true,
  },
  dots: ["#c9c6bf", "#97938a", "#161614"],
};

export default function MinimalLiquidTabs(props: LiquidTabsWorldProps) {
  return <LiquidBar look={look} {...props} />;
}
