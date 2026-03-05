// Card animation easing functions and sequencer

// ── Easing Functions ──

export function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function easeOutBounce(t: number): number {
  const n1 = 7.5625;
  const d1 = 2.75;
  if (t < 1 / d1) {
    return n1 * t * t;
  } else if (t < 2 / d1) {
    return n1 * (t -= 1.5 / d1) * t + 0.75;
  } else if (t < 2.5 / d1) {
    return n1 * (t -= 2.25 / d1) * t + 0.9375;
  } else {
    return n1 * (t -= 2.625 / d1) * t + 0.984375;
  }
}

export function easeOutBack(t: number): number {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
}

export function easeInOutCubic(t: number): number {
  return t < 0.5
    ? 4 * t * t * t
    : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

export function easeOutQuad(t: number): number {
  return 1 - (1 - t) * (1 - t);
}

export function linear(t: number): number {
  return t;
}

// ── Types ──

export type EasingFn = (t: number) => number;

export interface AnimationStep {
  duration: number; // milliseconds
  easing: EasingFn;
  onUpdate: (progress: number) => void;
  onComplete?: () => void;
}

export interface AnimationSequence {
  steps: AnimationStep[];
  onSequenceComplete?: () => void;
}

// ── Animation Runner ──

interface RunningAnimation {
  startTime: number;
  step: AnimationStep;
  stepIndex: number;
  sequence: AnimationSequence;
}

const runningAnimations = new Map<string, RunningAnimation>();
let animationFrame: number | null = null;

function tick() {
  const now = performance.now();
  const toRemove: string[] = [];

  for (const [id, anim] of runningAnimations) {
    const elapsed = now - anim.startTime;
    const rawProgress = Math.min(elapsed / anim.step.duration, 1);
    const easedProgress = anim.step.easing(rawProgress);

    anim.step.onUpdate(easedProgress);

    if (rawProgress >= 1) {
      anim.step.onComplete?.();

      const nextIndex = anim.stepIndex + 1;
      if (nextIndex < anim.sequence.steps.length) {
        // Advance to next step
        const nextStep = anim.sequence.steps[nextIndex];
        runningAnimations.set(id, {
          startTime: now,
          step: nextStep,
          stepIndex: nextIndex,
          sequence: anim.sequence,
        });
      } else {
        // Sequence complete
        anim.sequence.onSequenceComplete?.();
        toRemove.push(id);
      }
    }
  }

  for (const id of toRemove) {
    runningAnimations.delete(id);
  }

  if (runningAnimations.size > 0) {
    animationFrame = requestAnimationFrame(tick);
  } else {
    animationFrame = null;
  }
}

function ensureTicking() {
  if (animationFrame === null && runningAnimations.size > 0) {
    animationFrame = requestAnimationFrame(tick);
  }
}

let nextId = 0;

/** Start an animation sequence, returns an ID to cancel it */
export function startAnimation(sequence: AnimationSequence): string {
  if (sequence.steps.length === 0) {
    sequence.onSequenceComplete?.();
    return '';
  }
  const id = `anim_${nextId++}`;
  runningAnimations.set(id, {
    startTime: performance.now(),
    step: sequence.steps[0],
    stepIndex: 0,
    sequence,
  });
  ensureTicking();
  return id;
}

/** Cancel a running animation */
export function cancelAnimation(id: string): void {
  runningAnimations.delete(id);
}

// ── Lerp helpers ──

export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

export function lerpVec3(
  a: [number, number, number],
  b: [number, number, number],
  t: number
): [number, number, number] {
  return [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];
}

// ── Pre-built animation factories ──

/** Card draw: moves from a source position to target hand position with an arc */
export function createDrawAnimation(
  from: [number, number, number],
  to: [number, number, number],
  onUpdate: (pos: [number, number, number], scale: number) => void,
  onComplete?: () => void
): AnimationSequence {
  return {
    steps: [
      {
        duration: 400,
        easing: easeOutCubic,
        onUpdate: (t) => {
          const pos = lerpVec3(from, to, t);
          // Arc upward in the middle of the animation
          pos[1] += Math.sin(t * Math.PI) * 0.4;
          const scale = lerp(0.5, 1, t);
          onUpdate(pos, scale);
        },
        onComplete,
      },
    ],
    onSequenceComplete: onComplete,
  };
}

/** Card pick: rises up, then moves to drafted pile */
export function createPickAnimation(
  from: [number, number, number],
  to: [number, number, number],
  onUpdate: (pos: [number, number, number], scale: number, opacity: number) => void,
  onComplete?: () => void
): AnimationSequence {
  const mid: [number, number, number] = [from[0], from[1] + 0.5, from[2]];
  return {
    steps: [
      {
        duration: 250,
        easing: easeOutBack,
        onUpdate: (t) => {
          const pos = lerpVec3(from, mid, t);
          onUpdate(pos, lerp(1, 1.15, t), 1);
        },
      },
      {
        duration: 350,
        easing: easeOutCubic,
        onUpdate: (t) => {
          const pos = lerpVec3(mid, to, t);
          onUpdate(pos, lerp(1.15, 1, t), 1);
        },
        onComplete,
      },
    ],
    onSequenceComplete: onComplete,
  };
}

/** Card destroy: shrinks and fades away */
export function createDestroyAnimation(
  from: [number, number, number],
  onUpdate: (pos: [number, number, number], scale: number, opacity: number) => void,
  onComplete?: () => void
): AnimationSequence {
  return {
    steps: [
      {
        duration: 300,
        easing: easeOutCubic,
        onUpdate: (t) => {
          const pos: [number, number, number] = [from[0], from[1] + t * 0.3, from[2]];
          onUpdate(pos, lerp(1, 0, t), lerp(1, 0, t));
        },
        onComplete,
      },
    ],
    onSequenceComplete: onComplete,
  };
}

/** Card sell: glows then slides toward buyer position */
export function createSellAnimation(
  from: [number, number, number],
  buyerPos: [number, number, number],
  onUpdate: (pos: [number, number, number], scale: number, emissive: number) => void,
  onComplete?: () => void
): AnimationSequence {
  return {
    steps: [
      {
        duration: 300,
        easing: easeOutQuad,
        onUpdate: (t) => {
          // Glow phase
          onUpdate(from, 1, lerp(0, 0.8, t));
        },
      },
      {
        duration: 400,
        easing: easeOutCubic,
        onUpdate: (t) => {
          const pos = lerpVec3(from, buyerPos, t);
          pos[1] += Math.sin(t * Math.PI) * 0.2;
          onUpdate(pos, lerp(1, 0.3, t), lerp(0.8, 0, t));
        },
        onComplete,
      },
    ],
    onSequenceComplete: onComplete,
  };
}

/** Slide in from off-screen (used for new buyers) */
export function createSlideInAnimation(
  to: [number, number, number],
  direction: 'left' | 'right' | 'top',
  onUpdate: (pos: [number, number, number], scale: number) => void,
  onComplete?: () => void
): AnimationSequence {
  const from: [number, number, number] = [...to];
  if (direction === 'left') from[0] -= 3;
  else if (direction === 'right') from[0] += 3;
  else from[2] -= 3;

  return {
    steps: [
      {
        duration: 500,
        easing: easeOutBack,
        onUpdate: (t) => {
          const pos = lerpVec3(from, to, t);
          onUpdate(pos, lerp(0.8, 1, t));
        },
        onComplete,
      },
    ],
    onSequenceComplete: onComplete,
  };
}
