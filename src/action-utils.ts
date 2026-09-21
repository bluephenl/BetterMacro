import { Action, AppSettings, defaultAppSettings } from "./types";

export const actionDelay = (action: Action) => action.enabled === false ? 0 : action.kind === "wait" ? action.durationMs : action.delayMs;

export const actionDuration = (action: Action) => {
  if (action.enabled === false) return 0;
  if (action.kind === "wait") return action.durationMs;
  if (action.kind === "move") return action.durationMs === 0 ? action.delayMs : action.delayMs + action.durationMs;
  if (action.kind === "drag") return action.delayMs + action.durationMs;
  return action.delayMs;
};

export function normalizeAppSettings(value: unknown): AppSettings {
  if (!value || typeof value !== "object") return defaultAppSettings;
  const candidate = value as { pointerHz?: unknown; recording?: unknown };
  const recording = candidate.recording && typeof candidate.recording === "object"
    ? candidate.recording as Record<string, unknown>
    : {};
  const boolean = (key: keyof AppSettings["recording"]) => typeof recording[key] === "boolean"
    ? recording[key] as boolean
    : defaultAppSettings.recording[key];
  const pointerHz = candidate.pointerHz === 60 || candidate.pointerHz === 120 || candidate.pointerHz === 240
    ? candidate.pointerHz
    : defaultAppSettings.pointerHz;
  return {
    pointerHz,
    recording: {
      mouseClicks: boolean("mouseClicks"),
      mouseMovement: boolean("mouseMovement"),
      keyboard: boolean("keyboard"),
      timing: boolean("timing"),
      scrolling: boolean("scrolling"),
    },
  };
}

export function reorder<T>(items: T[], from: number, to: number): T[] {
  if (from < 0 || to < 0 || from >= items.length || to >= items.length) return items;
  const copy = [...items];
  const [item] = copy.splice(from, 1);
  copy.splice(to, 0, item);
  return copy;
}
