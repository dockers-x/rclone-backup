import { useAnimate, type Transition } from "motion/react";
import { useEffect, type CSSProperties } from "react";
import { GooFilter } from "@/components/shared/goo-filter";
import { useReducedMotionSafe } from "@/lib/motion";
import { cn } from "@/lib/utils";
import { STATUS_INDEX, type LiquidTabsWorldProps } from "./liquid-tabs";
import { useLiquid, type Liquid } from "./use-liquid";

/** How a world draws the tab bar. Minimal, Craft and Retro share the renderer and differ here. */
export type LiquidLook = {
  world: "minimal" | "craft" | "retro";
  track: CSSProperties;
  /** The indicator's colour, and its corner radius at rest. */
  blob: string;
  blobRadius: string;
  /** Text colour on and off the indicator. */
  on: string;
  off: string;
  tab: string;
  label: string;
  statusText: string;
  /** How the indicator travels between tabs. */
  liquid: Liquid;
  /** Status dot colours: offline, building, online. */
  dots: [string, string, string];
};

/**
 * The deploy status: a dot with two satellites under the goo filter. Building swings the satellites
 * out and back (the second a beat late) so they pull away from the dot; Online breathes the dot.
 * Leaving a state snaps back to rest, as a CSS animation does when it is removed.
 */
const STATUS_DOTS = [
  { size: 34, orbit: 0, delay: 0 },
  { size: 20, orbit: 38, delay: 0 },
  { size: 16, orbit: -38, delay: 0.3 },
];
const ORBIT: Transition = {
  duration: 1.2,
  ease: [0.5, 0, 0.5, 1],
  repeat: Infinity,
};
const BREATHE: Transition = {
  duration: 2.4,
  ease: "easeInOut",
  repeat: Infinity,
};

/**
 * Runs the loops from an effect rather than as mount animations: a parent `AnimatePresence
 * initial={false}` would otherwise skip them when the bar first renders already building.
 */
function StatusDots({
  status,
  colors,
  filterId,
}: {
  status: number;
  colors: LiquidLook["dots"];
  filterId: string;
}) {
  const reduced = useReducedMotionSafe();
  const [scope, run] = useAnimate<HTMLDivElement>();

  useEffect(() => {
    if (reduced) return;
    const dots = [...scope.current.children] as HTMLElement[];
    const loops =
      status === 1
        ? STATUS_DOTS.flatMap((d, j) =>
            d.orbit
              ? [
                  run(
                    dots[j],
                    { x: [0, d.orbit, 0] },
                    { ...ORBIT, delay: d.delay },
                  ),
                ]
              : [],
          )
        : status === 2
          ? [run(dots[0], { scale: [1, 1.18, 1] }, BREATHE)]
          : [];
    return () => {
      loops.forEach((l) => l.stop());
      run(dots, { x: 0, scale: 1 }, { duration: 0 });
    };
  }, [status, reduced, run, scope]);

  return (
    <div
      ref={scope}
      aria-hidden
      className="relative h-11 w-30"
      style={{ filter: `url(#${filterId})` }}
    >
      {STATUS_DOTS.map((d, j) => (
        <div
          key={j}
          className="absolute top-1/2 left-1/2 rounded-full transition-[background-color] duration-600 motion-reduce:transition-none"
          style={{
            width: d.size,
            height: d.size,
            margin: `${-d.size / 2}px 0 0 ${-d.size / 2}px`,
            backgroundColor: colors[status],
          }}
        />
      ))}
    </div>
  );
}

export function LiquidBar({
  look,
  ...p
}: LiquidTabsWorldProps & { look: LiquidLook }) {
  const reduced = useReducedMotionSafe();
  const n = p.tabs.length;
  const w = 100 / n;
  const st = p.status ? STATUS_INDEX[p.status] : 0;
  const columns = { gridTemplateColumns: `repeat(${n}, minmax(0, 1fr))` };
  // Where the pill rests, in shares of the row: CSS places it, so it is right before any script runs.
  const from = `${p.index * w}%`;
  const to = `${100 - (p.index + 1) * w}%`;

  const { box, rest, pieces, capL, body, capR, labels } = useLiquid({
    index: p.index,
    count: n,
    liquid: look.liquid,
    reduced,
  });

  const piece = { background: look.blob };
  const cap = { ...piece, borderRadius: look.liquid.round ? "999px" : 0 };

  return (
    <div
      data-ovio-world={look.world}
      className={cn(
        "flex w-full flex-col items-center gap-10 font-(family-name:--ovio-font) text-(--ovio-ink)",
        p.className,
      )}
    >
      {p.status && <GooFilter id={p.filterId} blur={p.goo} />}
      <div className="relative w-full max-w-135 p-1.5" style={look.track}>
        <div ref={box} aria-hidden className="absolute inset-1.5">
          <div
            ref={rest}
            className="absolute inset-y-0"
            style={{
              left: from,
              right: to,
              background: look.blob,
              borderRadius: look.blobRadius,
            }}
          />
          <div ref={pieces} className="invisible absolute inset-0">
            <div
              ref={body}
              className="absolute inset-y-0 left-0 origin-left"
              style={piece}
            />
            <div ref={capL} className="absolute inset-y-0 left-0" style={cap} />
            <div ref={capR} className="absolute inset-y-0 left-0" style={cap} />
          </div>
        </div>
        <div
          role="tablist"
          aria-label={p.label}
          onKeyDown={p.onKeyDown}
          className="relative grid"
          style={columns}
        >
          {p.tabs.map((tab, i) => {
            const active = i === p.index;
            return (
              <button
                key={`${i}:${tab}`}
                ref={p.tabRef(i)}
                id={p.tabId(i)}
                role="tab"
                type="button"
                aria-selected={active}
                aria-controls={p.panelId}
                tabIndex={active ? 0 : -1}
                onClick={() => p.select(i)}
                className={cn(
                  "cursor-pointer border-0 bg-transparent font-[inherit]",
                  look.tab,
                )}
                style={{ color: look.off }}
              >
                {tab}
              </button>
            );
          })}
        </div>
        {/* The labels again in the "on" colour, clipped to the pill: letters change colour exactly
            where it covers them, mid-move included. */}
        <div
          ref={labels}
          aria-hidden
          className="pointer-events-none absolute inset-1.5 grid"
          style={{
            ...columns,
            color: look.on,
            clipPath: `inset(0 ${to} 0 ${from} round ${look.blobRadius})`,
          }}
        >
          {p.tabs.map((tab, i) => (
            <span
              key={`${i}:${tab}`}
              data-liquid-label
              className={cn("block text-center", look.tab)}
            >
              {tab}
            </span>
          ))}
        </div>
      </div>

      {p.status && (
        <div className="flex flex-wrap items-center justify-center gap-5.5">
          <StatusDots status={st} colors={look.dots} filterId={p.filterId} />
          <div className="flex min-w-37.5 flex-col gap-0.5" role="status">
            <span className={cn("opacity-65", look.label)}>Deploy status</span>
            <span className={look.statusText}>{p.statusLabel}</span>
          </div>
        </div>
      )}

      {p.panel !== undefined && (
        <div
          id={p.panelId}
          role="tabpanel"
          aria-labelledby={p.tabId(p.index)}
          className={cn("opacity-60", look.label)}
        >
          {p.panel}
        </div>
      )}
    </div>
  );
}
