import {
  animate,
  motion,
  useMotionValue,
  useMotionTemplate,
  useTransform,
  useVelocity,
} from "motion/react";
import { useEffect, useRef, type PointerEvent } from "react";
import { motionTokens, useReducedMotionSafe } from "@/lib/motion";
import { cn } from "@/lib/utils";
import { STATUS_INDEX, type LiquidTabsWorldProps } from "../liquid-tabs";

const PAD = 8;
const DOTS = ["#a69e90", "#f4b52a", "#33b07a"];
const LABEL = "py-3.5 font-[inherit] text-[11px] font-extrabold sm:text-[15px]";

/**
 * Toy: the indicator is a plastic piece in a track. Click a tab, or drag and flick the piece;
 * it slides on a spring (k420 c20), squashes with its speed and snaps to the nearest tab.
 */
export default function ToyLiquidTabs(p: LiquidTabsWorldProps) {
  const reduced = useReducedMotionSafe();
  const n = p.tabs.length;
  const track = useRef<HTMLDivElement>(null);
  const x = useMotionValue(p.index);
  const v = useVelocity(x);
  const percent = useTransform(() => x.get() * 100);
  const translate = useMotionTemplate`${percent}%`;
  const squash = useTransform(() => Math.min(0.22, Math.abs(v.get()) * 0.035));
  const scaleX = useTransform(() => 1 + squash.get());
  const scaleY = useTransform(() => 1 - squash.get() * 0.6);
  // The labels in white, clipped to the piece: letters turn white exactly where it covers them,
  // while it slides and while it is dragged.
  const from = useTransform(() => (x.get() * 100) / n);
  const to = useTransform(() => 100 - ((x.get() + 1) * 100) / n);
  const clip = useMotionTemplate`inset(0 ${to}% 0 ${from}% round 12px)`;

  // The tab the piece is currently heading for, so external changes don't restart a flick.
  const heading = useRef(p.index);
  const drag = useRef<{
    start: number;
    moved: boolean;
    pointer: number;
  } | null>(null);

  const slideTo = (target: number, velocity = 0) => {
    heading.current = target;
    if (reduced || document.documentElement.dataset.input === "keyboard")
      x.set(target);
    else animate(x, target, { ...motionTokens.toy.slide, velocity });
  };

  const goTo = (target: number, velocity = 0) => {
    const t = Math.max(0, Math.min(n - 1, target));
    slideTo(t, velocity);
    p.select(t);
  };

  // Keyboard and controlled changes arrive as a new index: slide there with a small kick.
  useEffect(() => {
    if (drag.current || heading.current === p.index) return;
    const kick = Math.sign(p.index - x.get()) * 4;
    heading.current = p.index;
    if (reduced) x.set(p.index);
    else animate(x, p.index, { ...motionTokens.toy.slide, velocity: kick });
  }, [p.index, reduced, x, drag, heading]);

  const position = (clientX: number) => {
    const r = track.current!.getBoundingClientRect();
    const w = (r.width - PAD * 2) / n;
    return Math.max(
      -0.25,
      Math.min(n - 0.75, (clientX - r.left - PAD) / w - 0.5),
    );
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    if (!e.isPrimary || e.button !== 0) return;
    x.stop();
    drag.current = { start: e.clientX, moved: false, pointer: e.pointerId };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    if (!d.moved && Math.abs(e.clientX - d.start) > 4) d.moved = true;
    if (d.moved) x.set(position(e.clientX));
  };

  const onPointerUp = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    drag.current = null;
    if (!d.moved) return goTo(Math.round(position(d.start)));
    // Tabs per second, capped so a jumpy pointer cannot fling the piece across the whole bar.
    const velocity = Math.max(-12, Math.min(12, x.getVelocity()));
    goTo(Math.round(x.get() + velocity * 0.12), velocity);
  };

  const status = p.status ? STATUS_INDEX[p.status] : 0;

  return (
    <div
      data-ovio-world="toy"
      className={cn(
        "flex w-full flex-col items-center gap-10 font-(family-name:--ovio-font) text-(--ovio-ink)",
        p.className,
      )}
    >
      <div
        ref={track}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        className="relative w-full max-w-135 touch-none rounded-[18px] bg-(--ovio-track) p-2 select-none shadow-[inset_0_3px_7px_rgba(40,28,10,.32),0_1px_0_#fff]"
      >
        <motion.div
          aria-hidden
          className="absolute top-2 bottom-2 left-2 cursor-grab rounded-xl bg-(--ovio-red) shadow-[0_5px_0_var(--ovio-red-deep),0_10px_12px_-5px_rgba(40,28,10,.4),inset_0_2px_0_rgba(255,255,255,.3)]"
          style={{
            width: `calc((100% - ${PAD * 2}px) / ${n})`,
            x: translate,
            scaleX,
            scaleY,
          }}
        />
        <div
          role="tablist"
          aria-label={p.label}
          onKeyDown={p.onKeyDown}
          className="pointer-events-none relative grid"
          style={{ gridTemplateColumns: `repeat(${n}, minmax(0, 1fr))` }}
        >
          {p.tabs.map((tab, i) => (
            <button
              key={`${i}:${tab}`}
              ref={p.tabRef(i)}
              id={p.tabId(i)}
              role="tab"
              type="button"
              aria-selected={i === p.index}
              aria-controls={p.panelId}
              tabIndex={i === p.index ? 0 : -1}
              onClick={(e) => {
                // Pointer clicks are handled by the track; keyboard and assistive tech clicks land here.
                if (e.detail === 0) goTo(i);
              }}
              className={cn(
                LABEL,
                "pointer-events-auto cursor-pointer border-0 bg-transparent text-[#4a463e]",
              )}
            >
              {tab}
            </button>
          ))}
        </div>
        <motion.div
          aria-hidden
          className="pointer-events-none absolute inset-2 grid text-white"
          style={{
            gridTemplateColumns: `repeat(${n}, minmax(0, 1fr))`,
            clipPath: clip,
          }}
        >
          {p.tabs.map((tab, i) => (
            <span
              data-liquid-label
              key={`${i}:${tab}`}
              className={cn(LABEL, "block text-center")}
            >
              {tab}
            </span>
          ))}
        </motion.div>
      </div>

      {p.status && (
        <div className="flex flex-wrap items-center justify-center gap-4.5">
          <div
            aria-hidden
            className="relative h-10 w-37.5 rounded-full bg-(--ovio-track) p-1.25 shadow-[inset_0_3px_6px_rgba(40,28,10,.3)]"
          >
            <motion.div
              className="absolute top-1.25 left-1.25 flex h-7.5 w-11.5 items-center justify-center rounded-full bg-(--ovio-surface) shadow-[0_4px_0_#cfc8b8,0_7px_8px_-4px_rgba(40,28,10,.4)]"
              initial={false}
              animate={{ x: [0, 47, 94][status] }}
              transition={reduced ? { duration: 0 } : motionTokens.toy.piece}
            >
              <motion.span
                className="size-2.5 rounded-full shadow-[inset_0_1px_1px_rgba(0,0,0,.3)]"
                initial={false}
                animate={{ backgroundColor: DOTS[status] }}
                transition={{ duration: reduced ? 0 : 0.3 }}
              />
            </motion.div>
          </div>
          <div className="flex min-w-30 flex-col gap-0.5" role="status">
            <span className="font-(family-name:--ovio-mono) text-[11px] tracking-[0.08em] text-(--ovio-muted) uppercase">
              Deploy status
            </span>
            <span
              className="text-xl font-extrabold"
              style={{ fontStretch: "115%" }}
            >
              {p.statusLabel}
            </span>
          </div>
        </div>
      )}

      {p.panel !== undefined && (
        <div
          id={p.panelId}
          role="tabpanel"
          aria-labelledby={p.tabId(p.index)}
          className="font-(family-name:--ovio-mono) text-xs text-(--ovio-muted)"
        >
          {p.panel}
        </div>
      )}
    </div>
  );
}
