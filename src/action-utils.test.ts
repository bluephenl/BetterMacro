import { describe, expect, it } from "vitest";
import { actionDelay, actionDuration, normalizeAppSettings, reorder } from "./action-utils";

describe("action utilities", () => {
  it("uses wait duration as its timeline delay", () => expect(actionDelay({ kind: "wait", durationMs: 500 })).toBe(500));
  it("excludes disabled actions from timeline duration", () => expect(actionDelay({ kind: "wait", durationMs: 500, enabled: false })).toBe(0));
  it("includes pointer execution time in the macro duration", () => {
    expect(actionDuration({ kind: "move", x: 1, y: 2, durationMs: 300, delayMs: 100 })).toBe(400);
    expect(actionDuration({ kind: "move", x: 1, y: 2, durationMs: 0, delayMs: 100 })).toBe(100);
    expect(actionDuration({ kind: "drag", button: "left", points: [], durationMs: 300, delayMs: 100 })).toBe(400);
  });
  it("repairs malformed persisted settings", () => {
    expect(normalizeAppSettings({ pointerHz: 999, recording: { keyboard: false, timing: "yes" } })).toEqual({
      pointerHz: 120,
      recording: { mouseClicks: true, mouseMovement: false, keyboard: false, timing: true, scrolling: true },
    });
  });
  it("reorders an action without mutating the original array", () => {
    const actions = ["a", "b", "c"];
    expect(reorder(actions, 0, 2)).toEqual(["b", "c", "a"]);
    expect(actions).toEqual(["a", "b", "c"]);
  });
});
