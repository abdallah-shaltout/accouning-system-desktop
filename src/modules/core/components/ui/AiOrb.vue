<template>
    <div
        ref="rootEl"
        class="ai-orb"
        :class="[stateClass, { 'ai-orb--interactive': interactive, 'is-active': active, 'is-blinking': blinking }]"
        :data-theme="themeAttr"
        :style="{ width: `${size}px`, height: `${size}px` }"
        :role="interactive ? 'button' : 'img'"
        :tabindex="interactive ? 0 : undefined"
        :aria-pressed="interactive ? active : undefined"
        :aria-label="ariaLabel"
        @click="onClick"
        @keydown.enter.prevent="onKeydown"
        @keydown.space.prevent="onKeydown"
    >
        <div class="ai-orb__stage" :style="{ transform: `scale(${size / BASE_SIZE})` }">
            <div class="ai-orb__float">
                <div class="ai-orb__glow" />

                <div class="ai-orb__sphere" />

                <div class="ai-orb__eyes" :style="{ transform: `translate(${eye.x}px, ${eye.y}px)` }">
                    <div class="ai-orb__eye ai-orb__eye--left" />
                    <div class="ai-orb__eye ai-orb__eye--right" />
                </div>

                <div class="ai-orb__particles" aria-hidden="true">
                    <span class="ai-orb__particle ai-orb__particle--1" />
                    <span class="ai-orb__particle ai-orb__particle--2" />
                    <span class="ai-orb__particle ai-orb__particle--3" />
                    <span class="ai-orb__particle ai-orb__particle--4" />
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts">
export type AiOrbState = "idle" | "connecting" | "listening" | "thinking" | "speaking" | "muted" | "error";
</script>

<script setup lang="ts">
/**
 * Self-contained AI voice-assistant orb (no app imports, no Tailwind) so it can
 * be copied into other projects as a single file. Reacts to a live
 * MediaStream, an external 0–1 level, or a simulated speech pattern.
 *
 * Two modes:
 *  - legacy (no `state` prop): today's active/idle toggle behaviour, unchanged.
 *  - state mode (`state` set): drives eyes/glow/level from one of seven voice states.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

const BASE_SIZE = 200;
// Mic hiss / room noise sits around this RMS — anything below it shouldn't move the eyes.
const NOISE_FLOOR = 0.01;

const props = withDefaults(
    defineProps<{
        /** Live audio to react to — a mic stream from getUserMedia or a remote track. Legacy alias for `inputStream`. */
        stream?: MediaStream | null;
        /** External 0–1 level (e.g. from your own analyser). Takes priority over `stream`. */
        volume?: number | null;
        /** Fake a speech pattern while active when neither `stream` nor `volume` is given. */
        simulate?: boolean;
        /** Mic loudness multiplier — raise it if the eyes barely move, lower it if they stay maxed. */
        sensitivity?: number;
        size?: number;
        ariaLabel?: string;
        /** When set, drives behaviour via the state table; `active` is derived from it. */
        state?: AiOrbState | null;
        /** The user's mic — same role as `stream`, named for state mode. */
        inputStream?: MediaStream | null;
        /** The assistant's voice output. */
        outputStream?: MediaStream | null;
        /** `auto` follows an ancestor `.dark` class; `light`/`dark` force it. */
        theme?: "auto" | "light" | "dark";
        /** false = decorative only (no click/keyboard, role="img"). Voice mode passes false. */
        interactive?: boolean;
    }>(),
    {
        stream: null,
        volume: null,
        simulate: true,
        sensitivity: 5,
        size: 200,
        ariaLabel: "AI voice assistant",
        state: null,
        inputStream: null,
        outputStream: null,
        theme: "auto",
        interactive: true,
    },
);

const active = defineModel<boolean>("active", { default: false });

const rootEl = ref<HTMLElement | null>(null);
const level = ref(0);
const eye = ref({ x: 0, y: 0 });
const blinking = ref(false);

const themeAttr = computed(() => (props.theme === "auto" ? undefined : props.theme));

// ── Mode ─────────────────────────────────────────────────────────────────
const isStateMode = computed(() => props.state != null);
const stateClass = computed(() => (isStateMode.value ? `ai-orb--${props.state}` : "ai-orb--legacy"));

// `active` mirrors the state in state mode, still toggled by click in legacy mode.
watch(
    () => props.state,
    (s) => {
        if (s != null) active.value = s === "listening" || s === "thinking" || s === "speaking";
    },
    { immediate: true },
);

function onClick() {
    if (!props.interactive || isStateMode.value) return;
    active.value = !active.value;
}
function onKeydown() {
    onClick();
}

// ── Audio meters ─────────────────────────────────────────────────────────
let audioCtx: AudioContext | null = null;
interface Analyser {
    node: AnalyserNode;
    source: MediaStreamAudioSourceNode;
    samples: Float32Array;
}
let inputAnalyser: Analyser | null = null;
let outputAnalyser: Analyser | null = null;

function ensureContext(): AudioContext {
    if (!audioCtx) audioCtx = new AudioContext();
    void audioCtx.resume();
    return audioCtx;
}

function createAnalyser(ctx: AudioContext, stream: MediaStream): Analyser {
    const source = ctx.createMediaStreamSource(stream);
    const node = ctx.createAnalyser();
    node.fftSize = 512;
    // Not connected to ctx.destination — we only measure, never play back (no echo/double playback).
    // Note: Chrome only feeds a remote WebRTC stream into Web Audio while that
    // stream is also attached to a playing <audio> element elsewhere (the SDK's
    // own hidden element does this for outputStream — never detach it).
    source.connect(node);
    return { node, source, samples: new Float32Array(node.fftSize) };
}

function readAnalyser(a: Analyser | null): number {
    if (!a) return 0;
    a.node.getFloatTimeDomainData(a.samples as Float32Array<ArrayBuffer>);
    let sum = 0;
    for (let i = 0; i < a.samples.length; i++) sum += a.samples[i] * a.samples[i];
    return Math.sqrt(sum / a.samples.length);
}

function attachInput(stream: MediaStream | null) {
    inputAnalyser?.source.disconnect();
    inputAnalyser = stream && stream.getAudioTracks().length > 0 ? createAnalyser(ensureContext(), stream) : null;
}
function attachOutput(stream: MediaStream | null) {
    outputAnalyser?.source.disconnect();
    outputAnalyser = stream && stream.getAudioTracks().length > 0 ? createAnalyser(ensureContext(), stream) : null;
}

const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

function meterLevel(a: Analyser | null): number {
    return clamp01((readAnalyser(a) - NOISE_FLOOR) * props.sensitivity);
}

function nextTarget(): number {
    if (isStateMode.value) {
        switch (props.state) {
            case "listening":
                return meterLevel(inputAnalyser) * 0.6;
            case "speaking":
                return meterLevel(outputAnalyser);
            default:
                return 0;
        }
    }
    if (props.volume != null) return clamp01(props.volume);
    if (inputAnalyser) return meterLevel(inputAnalyser);
    if (props.simulate) {
        // Bursts of sound with short pauses, like natural speech.
        return Math.random() > 0.4 ? Math.random() * 0.8 + 0.2 : 0.05;
    }
    return 0;
}

// ── rAF loop — writes CSS vars directly, no Vue re-render per frame ───────
let rafId = 0;
let rafRunning = false;

function frame() {
    const target = nextTarget();
    // Fast attack (mouth opens), slower decay (mouth closes).
    const smoothing = target > level.value ? 0.6 : 0.3;
    level.value += (target - level.value) * smoothing;
    rootEl.value?.style.setProperty("--orb-level", level.value.toFixed(3));
    rafId = requestAnimationFrame(frame);
}

function canLevelBeNonZero(): boolean {
    if (isStateMode.value) return props.state === "listening" || props.state === "speaking";
    return active.value;
}

function updateRaf() {
    const shouldRun = canLevelBeNonZero() && !document.hidden;
    if (shouldRun && !rafRunning) {
        rafRunning = true;
        rafId = requestAnimationFrame(frame);
    } else if (!shouldRun && rafRunning) {
        rafRunning = false;
        cancelAnimationFrame(rafId);
        rafId = 0;
        if (!canLevelBeNonZero()) {
            level.value = 0;
            rootEl.value?.style.setProperty("--orb-level", "0");
        }
    }
}

function onVisibilityChange() {
    updateRaf();
}

// ── "Life": blinking and looking around ──────────────────────────────────
let lifeTimer: ReturnType<typeof setTimeout> | undefined;
let blinkTimer: ReturnType<typeof setTimeout> | undefined;
let reducedMotion = false;

function liveLegacy() {
    if (Math.random() > 0.7 && !blinking.value) {
        blinking.value = true;
        clearTimeout(blinkTimer);
        blinkTimer = setTimeout(() => (blinking.value = false), 150);
    }
    // While active it's talking to the user, so it glances around less.
    const shouldMove = !reducedMotion && (active.value ? Math.random() > 0.6 : Math.random() > 0.3);
    if (shouldMove) {
        const range = 20;
        eye.value = { x: Math.random() * range * 2 - range, y: Math.random() * range - range / 2 };
    } else if (Math.random() > 0.7) {
        eye.value = { x: 0, y: 0 };
    }
    lifeTimer = setTimeout(liveLegacy, 1000 + Math.random() * 2500);
}

function blinkOnce(duration = 150) {
    if (blinking.value) return;
    blinking.value = true;
    clearTimeout(blinkTimer);
    blinkTimer = setTimeout(() => (blinking.value = false), duration);
}

function liveState() {
    if (props.state !== "muted") {
        if (Math.random() > 0.7) blinkOnce();
    }

    switch (props.state) {
        case "idle": {
            const shouldMove = !reducedMotion && Math.random() > 0.3;
            if (shouldMove) {
                const range = 20;
                eye.value = { x: Math.random() * range * 2 - range, y: Math.random() * range - range / 2 };
            } else if (Math.random() > 0.7) {
                eye.value = { x: 0, y: 0 };
            }
            break;
        }
        case "connecting": {
            if (!reducedMotion) {
                eye.value = { x: eye.value.x >= 0 ? -12 : 12, y: 0 };
            } else {
                eye.value = { x: 0, y: 0 };
            }
            break;
        }
        case "listening": {
            const shouldMove = !reducedMotion && Math.random() > 0.8;
            if (shouldMove) {
                const range = 8;
                eye.value = { x: Math.random() * range * 2 - range, y: Math.random() * range - range / 2 };
            } else if (Math.random() > 0.5) {
                eye.value = { x: 0, y: 0 };
            }
            break;
        }
        case "thinking": {
            if (!reducedMotion) {
                eye.value = { x: eye.value.x >= 0 ? -10 : 10, y: -10 };
            } else {
                eye.value = { x: 0, y: -10 };
            }
            break;
        }
        case "speaking": {
            const shouldMove = !reducedMotion && Math.random() > 0.6;
            if (shouldMove) {
                const range = 20;
                eye.value = { x: Math.random() * range * 2 - range, y: Math.random() * range - range / 2 };
            } else if (Math.random() > 0.7) {
                eye.value = { x: 0, y: 0 };
            }
            break;
        }
        case "muted":
        case "error":
        default:
            eye.value = { x: 0, y: 0 };
            break;
    }

    const interval = props.state === "connecting" ? 700 : props.state === "thinking" ? 1200 : 1000 + Math.random() * 2500;
    lifeTimer = setTimeout(liveState, interval);
}

function restartLife() {
    clearTimeout(lifeTimer);
    if (isStateMode.value) liveState();
    else liveLegacy();
}

onMounted(() => {
    reducedMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

    // Legacy `stream` prop maps onto the input analyser.
    watch(
        () => (isStateMode.value ? props.inputStream : props.stream),
        attachInput,
        { immediate: true },
    );
    watch(() => props.outputStream, attachOutput, { immediate: true });

    watch(active, updateRaf, { immediate: true });
    watch(() => props.state, updateRaf);

    watch(
        () => props.state,
        () => restartLife(),
    );
    restartLife();

    document.addEventListener("visibilitychange", onVisibilityChange);
});

onBeforeUnmount(() => {
    cancelAnimationFrame(rafId);
    rafRunning = false;
    clearTimeout(lifeTimer);
    clearTimeout(blinkTimer);
    inputAnalyser?.source.disconnect();
    outputAnalyser?.source.disconnect();
    inputAnalyser = null;
    outputAnalyser = null;
    if (audioCtx) {
        void audioCtx.close();
        audioCtx = null;
    }
    document.removeEventListener("visibilitychange", onVisibilityChange);
});

defineExpose({});
</script>

<style scoped>
.ai-orb {
    position: relative;
    flex-shrink: 0;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
    --orb-level: 0;

    /* Light theme tokens (default) */
    --orb-sphere: radial-gradient(circle at 25% 25%, #a5f3fc 0%, #60a5fa 18%, #7c3aed 55%, #3b0764 92%);
    --orb-inset: inset -18px -18px 36px rgba(59, 7, 100, 0.45), inset 10px 10px 30px rgba(255, 255, 255, 0.45);
    --orb-glow-color: #a855f7;
    --orb-glow-base: 0.28;
    --orb-glow-gain: 0.3;
    --orb-glow-error: #ef4444;
    --orb-particle-color: #8b5cf6;
    --orb-particle-shadow: 0 0 6px rgba(139, 92, 246, 0.6);
    --orb-eye-glow: 0 0 10px rgba(255, 255, 255, 0.7);
    --orb-ground: rgba(76, 29, 149, 0.18);
    --orb-focus: #7c3aed;
}

.ai-orb--interactive {
    cursor: pointer;
}

:global(.dark) .ai-orb,
.ai-orb[data-theme="dark"] {
    --orb-sphere: radial-gradient(circle at 25% 25%, #22d3ee 0%, #3b82f6 15%, #4c1d95 50%, #09090b 85%);
    --orb-inset: inset -20px -20px 40px rgba(0, 0, 0, 0.8), inset 10px 10px 30px rgba(255, 255, 255, 0.2);
    --orb-glow-color: #9333ea;
    --orb-glow-base: 0.4;
    --orb-glow-gain: 0.4;
    --orb-glow-error: #f87171;
    --orb-particle-color: #d8b4fe;
    --orb-particle-shadow: 0 0 8px #d8b4fe;
    --orb-eye-glow: 0 0 10px rgba(255, 255, 255, 0.5);
    --orb-ground: transparent;
    --orb-focus: #22d3ee;
}

.ai-orb[data-theme="light"] {
    --orb-sphere: radial-gradient(circle at 25% 25%, #a5f3fc 0%, #60a5fa 18%, #7c3aed 55%, #3b0764 92%);
    --orb-inset: inset -18px -18px 36px rgba(59, 7, 100, 0.45), inset 10px 10px 30px rgba(255, 255, 255, 0.45);
    --orb-glow-color: #a855f7;
    --orb-glow-base: 0.28;
    --orb-glow-gain: 0.3;
    --orb-glow-error: #ef4444;
    --orb-particle-color: #8b5cf6;
    --orb-particle-shadow: 0 0 6px rgba(139, 92, 246, 0.6);
    --orb-eye-glow: 0 0 10px rgba(255, 255, 255, 0.7);
    --orb-ground: rgba(76, 29, 149, 0.18);
    --orb-focus: #7c3aed;
}

.ai-orb:focus-visible {
    outline: 2px solid var(--orb-focus);
    outline-offset: 4px;
    border-radius: 50%;
}

/* Everything inside is authored at 200px and scaled as one unit, so every
   size keeps the exact same proportions and motion. */
.ai-orb__stage {
    position: absolute;
    top: 0;
    left: 0;
    width: 200px;
    height: 200px;
    transform-origin: 0 0;
}

/* Light-only ground shadow: depth on white without a black edge. */
.ai-orb__stage::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: -6px;
    width: 60%;
    height: 10%;
    transform: translateX(-50%);
    background: var(--orb-ground);
    filter: blur(10px);
    border-radius: 50%;
    z-index: 0;
    animation: ai-orb-ground-float 6s ease-in-out infinite;
}

:global(.dark) .ai-orb__stage::after,
.ai-orb[data-theme="dark"] .ai-orb__stage::after {
    display: none;
}

.ai-orb__float {
    position: relative;
    width: 100%;
    height: 100%;
    animation: ai-orb-float 6s ease-in-out infinite;
}

.ai-orb__sphere {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--orb-sphere);
    box-shadow: var(--orb-inset);
    z-index: 10;
}

.ai-orb__glow {
    position: absolute;
    top: 50%;
    left: 50%;
    width: calc(100% + var(--orb-level) * 80%);
    height: calc(100% + var(--orb-level) * 80%);
    transform: translate(-50%, -50%);
    border-radius: 50%;
    background: var(--orb-glow-color);
    opacity: calc(var(--orb-glow-base) + var(--orb-level) * var(--orb-glow-gain));
    filter: blur(40px);
    z-index: 1;
    transition: background-color 0.2s ease;
}

.ai-orb__eyes {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 24px;
    transition: transform 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.ai-orb__eye {
    width: 12px;
    height: 40px;
    background-color: #fff;
    border-radius: 10px;
    box-shadow: var(--orb-eye-glow);
    transform-origin: center;
    transition: transform 0.05s linear;
}

/* Legacy active bounce (no `state`) */
.ai-orb--legacy.is-active .ai-orb__eye--left {
    transform: scaleY(calc(1 + var(--orb-level) * 1.5));
}
.ai-orb--legacy.is-active .ai-orb__eye--right {
    transform: scaleY(calc(1 + var(--orb-level) * 1.2));
}

/* State-mode per-state eye shape */
.ai-orb--speaking .ai-orb__eye--left {
    transform: scaleY(calc(1 + var(--orb-level) * 1.5));
}
.ai-orb--speaking .ai-orb__eye--right {
    transform: scaleY(calc(1 + var(--orb-level) * 1.2));
}
.ai-orb--listening .ai-orb__eye--left,
.ai-orb--listening .ai-orb__eye--right {
    transform: scaleY(calc(1 + var(--orb-level) * 0.8));
}
.ai-orb--thinking .ai-orb__eye {
    transform: scaleY(0.7);
}
.ai-orb--muted .ai-orb__eye {
    transform: scaleY(0.35);
}
.ai-orb--error .ai-orb__eye {
    transform: scaleY(0.5);
}

/* Blink overrides everything except muted (already half-closed) */
.ai-orb.is-blinking:not(.ai-orb--muted) .ai-orb__eye {
    transform: scaleY(0.05) !important;
    transition: transform 0.1s cubic-bezier(0.4, 0, 1, 1);
}

/* Glow color for error */
.ai-orb--error .ai-orb__glow {
    background: var(--orb-glow-error);
}

/* Glow dim for muted */
.ai-orb--muted .ai-orb__glow {
    opacity: calc(var(--orb-glow-base) * 0.6);
}

/* Connecting breathing pulse */
.ai-orb--connecting .ai-orb__glow {
    animation: ai-orb-breathe 1.4s ease-in-out infinite;
}

/* Thinking slow pulse */
.ai-orb--thinking .ai-orb__glow {
    animation: ai-orb-breathe-slow 2.4s ease-in-out infinite;
}

.ai-orb__particles {
    position: absolute;
    inset: 0;
    z-index: 15;
    pointer-events: none;
}

.ai-orb__particle {
    position: absolute;
    width: 6px;
    height: 6px;
    background-color: var(--orb-particle-color);
    border-radius: 50%;
    filter: blur(1px);
    box-shadow: var(--orb-particle-shadow);
    z-index: 5;
}

.ai-orb__particle--1 {
    top: 20%;
    left: 20%;
    animation: ai-orb-drift-1 3s infinite linear;
}

.ai-orb__particle--2 {
    top: 70%;
    left: 80%;
    animation: ai-orb-drift-2 4s infinite linear 1s;
}

.ai-orb__particle--3 {
    top: 80%;
    left: 30%;
    animation: ai-orb-drift-3 3.5s infinite linear 0.5s;
}

.ai-orb__particle--4 {
    top: 30%;
    left: 70%;
    animation: ai-orb-drift-4 4.5s infinite linear 2s;
}

@keyframes ai-orb-float {
    0%,
    100% {
        transform: translateY(0);
    }
    50% {
        transform: translateY(-15px);
    }
}

@keyframes ai-orb-ground-float {
    0%,
    100% {
        transform: translateX(-50%) scale(1);
        opacity: 1;
    }
    50% {
        transform: translateX(-50%) scale(0.85);
        opacity: 0.7;
    }
}

@keyframes ai-orb-breathe {
    0%,
    100% {
        opacity: var(--orb-glow-base);
    }
    50% {
        opacity: calc(var(--orb-glow-base) + 0.3);
    }
}

@keyframes ai-orb-breathe-slow {
    0%,
    100% {
        opacity: var(--orb-glow-base);
    }
    50% {
        opacity: calc(var(--orb-glow-base) + 0.15);
    }
}

@keyframes ai-orb-drift-1 {
    0% {
        transform: translate(0, 0) scale(1);
        opacity: 0;
    }
    50% {
        opacity: 0.8;
    }
    100% {
        transform: translate(-40px, -60px) scale(0.5);
        opacity: 0;
    }
}

@keyframes ai-orb-drift-2 {
    0% {
        transform: translate(0, 0) scale(1);
        opacity: 0;
    }
    50% {
        opacity: 0.6;
    }
    100% {
        transform: translate(50px, -40px) scale(1.5);
        opacity: 0;
    }
}

@keyframes ai-orb-drift-3 {
    0% {
        transform: translate(0, 0) scale(0.8);
        opacity: 0;
    }
    50% {
        opacity: 0.9;
    }
    100% {
        transform: translate(-30px, 50px) scale(0.2);
        opacity: 0;
    }
}

@keyframes ai-orb-drift-4 {
    0% {
        transform: translate(0, 0) scale(1.2);
        opacity: 0;
    }
    50% {
        opacity: 0.5;
    }
    100% {
        transform: translate(60px, 40px) scale(0.8);
        opacity: 0;
    }
}

@media (prefers-reduced-motion: reduce) {
    .ai-orb__float,
    .ai-orb__particle,
    .ai-orb__stage::after {
        animation: none;
    }
    .ai-orb__particle {
        opacity: 0.6;
    }
    .ai-orb__eyes {
        transition: none;
    }
    .ai-orb__glow {
        opacity: var(--orb-glow-base);
    }
    .ai-orb--connecting .ai-orb__glow,
    .ai-orb--thinking .ai-orb__glow {
        animation: none;
        opacity: calc(var(--orb-glow-base) + 0.15);
    }
    /* The level-driven bounce stays (tells you who is speaking), but capped at 50% amplitude. */
    .ai-orb--speaking .ai-orb__eye--left,
    .ai-orb--legacy.is-active .ai-orb__eye--left {
        transform: scaleY(calc(1 + var(--orb-level) * 0.75));
    }
    .ai-orb--speaking .ai-orb__eye--right,
    .ai-orb--legacy.is-active .ai-orb__eye--right {
        transform: scaleY(calc(1 + var(--orb-level) * 0.6));
    }
    .ai-orb--listening .ai-orb__eye--left,
    .ai-orb--listening .ai-orb__eye--right {
        transform: scaleY(calc(1 + var(--orb-level) * 0.4));
    }
}
</style>
