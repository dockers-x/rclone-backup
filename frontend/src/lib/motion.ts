"use client";

import { useReducedMotion, type Transition } from "motion/react";

/**
 * Ovio motion tokens. Each world moves for a different reason:
 * Minimal moves to show state, Craft settles like paper, Retro steps like old hardware,
 * Toy is driven by physics. Springs are written as stiffness (k) and damping (c), mass 1.
 */

/**
 * How a data component animates: "enter-exit" plays its entrance, "always" adds an idle loop
 * (a live marker, a flicker), "none" draws it still. Reduced motion forces "none".
 */
export type OvioAnimation = "none" | "enter-exit" | "always";

export type Ease = [number, number, number, number] | ((t: number) => number);

/** CSS steps(n, end) as a Motion easing function. */
export function steps(n: number): (t: number) => number {
  return (t) => (t >= 1 ? 1 : Math.floor(t * n) / n);
}

export const ease = {
  /** Minimal: quick, decisive, no overshoot. */
  minimal: [0.2, 0, 0, 1],
  /** Minimal goo trail: slow out, fast in. */
  glide: [0.6, 0, 0.2, 1],
  /** Craft: paper lifts and settles with a soft overshoot. */
  craft: [0.34, 1.5, 0.5, 1],
  /** Stage entrance used when switching worlds. */
  stage: [0.2, 0.7, 0.2, 1],
} as const satisfies Record<string, Ease>;

const spring = (stiffness: number, damping: number, mass = 1) =>
  ({ type: "spring", stiffness, damping, mass }) as const;

export const motionTokens = {
  minimal: {
    fast: { duration: 0.08, ease: ease.minimal },
    base: { duration: 0.14, ease: ease.minimal },
    slow: { duration: 0.2, ease: ease.minimal },
  },
  craft: {
    base: { duration: 0.3, ease: ease.craft },
    slow: { duration: 0.32, ease: ease.craft },
  },
  retro: {
    frames: (n: number, duration: number) => ({ duration, ease: steps(n) }),
    /** 1s blink: on for half, off for half. Animate opacity [1, 0]. */
    blink: {
      duration: 0.5,
      repeat: Infinity,
      repeatType: "reverse",
      ease: steps(1),
    },
  },
  toy: {
    /** Rotary knob: heavy, slow to settle. */
    knob: spring(150, 28),
    /** Raised key: stiff, fast return. */
    key: spring(900, 22),
    /** A loose piece that wobbles back to its seat. */
    piece: spring(360, 17),
    /** A piece sliding in a track. */
    slide: spring(600, 24),
  },
} as const;

/** Reduced motion: movement resolves at once, but opacity still fades, so changes don't flash. */
const REDUCED: Transition = {
  duration: 0,
  opacity: { duration: 0.15, ease: ease.minimal },
};

export function useReducedMotionSafe(): boolean {
  return (
    (useReducedMotion() ?? false) ||
    (typeof document !== "undefined" &&
      document.documentElement.dataset.input === "keyboard")
  );
}

/** Returns the transition or, under prefers-reduced-motion, one that only fades (springs resolve instantly). */
export function useOvioTransition(transition: Transition): Transition {
  return useReducedMotionSafe() ? REDUCED : transition;
}
