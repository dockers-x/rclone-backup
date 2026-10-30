"use client";

import { calcGeneratorDuration, spring } from "motion";
import { useLayoutEffect, useRef } from "react";

type Spring = { stiffness: number; damping: number };

/**
 * How the indicator travels. Its two edges are separate springs: the one leading the move is stiff
 * and pulls ahead, the other is softer and follows, so the pill stretches toward the new tab and
 * gathers itself up behind, like a drop of liquid.
 */
export type Liquid = {
  lead: Spring;
  trail: Spring;
  /** How much thinner the pill gets per tab of stretch, as a share of its height. */
  squash: number;
  /** Rounded caps (a stadium) or square ones. */
  round: boolean;
  /** For a stepped look: frames per second, and the pixel grid edges land on. */
  fps?: number;
  snap?: number;
};

type Run = {
  gL: ReturnType<typeof spring>;
  gR: ReturnType<typeof spring>;
  t0: number;
  anims: Animation[];
};

/** Longest a move may take, in ms, whatever the springs say. */
const MAX = 1500;

/**
 * Moves the indicator between tabs. Both edge springs are sampled up front and played as native
 * keyframe animations on transform (and on the labels' clip-path), so the pill keeps moving while
 * the page is busy. A new tab mid-move restarts from where the edges are, with their velocity.
 * At rest the CSS pill shows instead: the moving pieces end in exactly its shape, so the swap
 * can't be seen.
 */
export function useLiquid({
  index,
  count,
  liquid,
  reduced,
}: {
  index: number;
  count: number;
  liquid: Liquid;
  reduced: boolean;
}) {
  // The row the pill moves in; the pill at rest, placed by CSS; the moving pill, a wrapper that
  // squashes around a left cap, a body and a right cap; and the labels in the "on" colour.
  const box = useRef<HTMLDivElement>(null);
  const rest = useRef<HTMLDivElement>(null);
  const pieces = useRef<HTMLDivElement>(null);
  const capL = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const capR = useRef<HTMLDivElement>(null);
  const labels = useRef<HTMLDivElement>(null);
  const state = useRef({ from: index, W: 0, H: 0, run: null as Run | null });

  // The row's size, for the pixel geometry the moving pieces need.
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const s = state.current;
      s.W = el.clientWidth;
      s.H = el.clientHeight;
      if (body.current) body.current.style.width = `${s.W / count}px`;
      for (const cap of [capL.current, capR.current])
        if (cap) cap.style.width = `${s.H}px`;
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [count]);

  useLayoutEffect(() => {
    const s = state.current;
    const prev = s.from;
    s.from = index;
    if (prev === index) return;

    const settle = () => {
      if (rest.current) rest.current.style.visibility = "";
      if (pieces.current) pieces.current.style.visibility = "hidden";
      s.run?.anims.forEach((a) => a.cancel());
      s.run = null;
    };
    if (reduced || !s.W) {
      settle();
      return;
    }

    const { W, H } = s;
    const tab = W / count;
    const now = performance.now();
    // Start from where the edges are: mid-move, sampled with their velocity; at rest, the old tab.
    let L0 = prev * tab;
    let R0 = (prev + 1) * tab;
    let vL = 0;
    let vR = 0;
    if (s.run) {
      const t = now - s.run.t0;
      L0 = s.run.gL.next(t).value;
      R0 = s.run.gR.next(t).value;
      vL = s.run.gL.velocity?.(t) ?? 0;
      vR = s.run.gR.velocity?.(t) ?? 0;
    }
    const L1 = index * tab;
    const R1 = (index + 1) * tab;
    const toRight = L1 > L0;
    const edge = (from: number, to: number, velocity: number, leads: boolean) =>
      spring({
        keyframes: [from, to],
        velocity,
        ...(leads ? liquid.lead : liquid.trail),
        restDelta: 0.3,
        restSpeed: 8,
      });
    const gL = edge(L0, L1, vL, !toRight);
    const gR = edge(R0, R1, vR, toRight);
    const duration = Math.min(
      MAX,
      Math.max(calcGeneratorDuration(gL), calcGeneratorDuration(gR)),
    );

    // One keyframe per frame; a stepped world jumps to each as its frame begins and holds it.
    const dt = 1000 / (liquid.fps ?? 60);
    const frames = Math.max(2, Math.ceil(duration / dt) + 1);
    const snap = (v: number) =>
      liquid.snap ? Math.round(v / liquid.snap) * liquid.snap : v;
    const easing = liquid.fps ? "steps(1, start)" : "linear";
    const leftCap: Keyframe[] = [];
    const rightCap: Keyframe[] = [];
    const middle: Keyframe[] = [];
    const squash: Keyframe[] = [];
    const clip: Keyframe[] = [];
    for (let k = 0; k < frames; k++) {
      const t = Math.min(duration, k * dt);
      // The last frame lands exactly on the resting pill, grid or not.
      const last = k === frames - 1;
      const L = last ? L1 : snap(gL.next(t).value);
      const R = last ? R1 : Math.max(L + H, snap(gR.next(t).value));
      // Thinner the further it stretches, never below 80% of its height.
      const sy = Math.max(
        0.8,
        1 - liquid.squash * Math.max(0, (R - L) / tab - 1),
      );
      const inset = (H * (1 - sy)) / 2;
      const radius = liquid.round ? (H / 2) * sy : 0;
      leftCap.push({ transform: `translateX(${L}px)`, easing });
      rightCap.push({ transform: `translateX(${R - H}px)`, easing });
      // The body runs between the caps' centres, under them, so no seam shows.
      middle.push({
        transform: `translateX(${L + H / 2}px) scaleX(${(R - L - H) / tab})`,
        easing,
      });
      squash.push({ transform: `scaleY(${sy})`, easing });
      clip.push({
        clipPath: `inset(${inset}px ${W - R}px ${inset}px ${L}px round ${radius}px)`,
        easing,
      });
    }

    const timing: KeyframeAnimationOptions = {
      duration: (frames - 1) * dt,
      fill: "both",
    };
    const anims = [
      capL.current?.animate(leftCap, timing),
      capR.current?.animate(rightCap, timing),
      body.current?.animate(middle, timing),
      pieces.current?.animate(squash, timing),
      labels.current?.animate(clip, timing),
    ].filter((a): a is Animation => !!a);

    // The new run takes over before the old one is dropped, so no frame shows neither.
    s.run?.anims.forEach((a) => a.cancel());
    const run: Run = { gL, gR, t0: now, anims };
    s.run = run;
    if (pieces.current) pieces.current.style.visibility = "visible";
    if (rest.current) rest.current.style.visibility = "hidden";
    anims[0]?.finished.then(
      () => s.run === run && settle(),
      () => {},
    );
  }, [index, count, liquid, reduced]);

  useLayoutEffect(() => {
    const s = state.current;
    return () => s.run?.anims.forEach((a) => a.cancel());
  }, []);

  return { box, rest, pieces, capL, body, capR, labels };
}
