export type MouseButton = "left" | "right" | "center" | "other";
export type ScrollUnit = "pixel" | "line";
export type PointerSample = { x: number; y: number; offsetMs: number; screenEdge?: boolean };
export type ScreenPoint = { x: number; y: number };
export type RelativePoint = { x: number; y: number };
export type SemanticNode = {
  role: string;
  subrole?: string | null;
  identifier?: string | null;
  title?: string | null;
  description?: string | null;
  help?: string | null;
  value?: string | null;
  filename?: string | null;
};
export type SemanticTarget = {
  application?: string | null;
  role: string;
  subrole?: string | null;
  identifier?: string | null;
  title?: string | null;
  description?: string | null;
  help?: string | null;
  value?: string | null;
  filename?: string | null;
  actions?: string[];
  path?: number[];
  activationPoint?: RelativePoint | null;
  window?: SemanticNode | null;
  ancestors?: SemanticNode[];
};
export type SystemCommand = "missionControl" | "showDesktop" | "revealDock" | "desktopAndDock";
export type Action = { enabled?: boolean } & (
  | { kind: "click"; x: number; y: number; button: MouseButton; clicks: number; delayMs: number; approach?: PointerSample[]; target?: SemanticTarget | null }
  | { kind: "move"; x: number; y: number; durationMs: number; delayMs: number; screenEdge?: boolean }
  | { kind: "drag"; button: MouseButton; approach?: PointerSample[]; points: PointerSample[]; durationMs: number; delayMs: number }
  | { kind: "scroll"; x: number; y: number; unit?: ScrollUnit; delayMs: number; at?: ScreenPoint | null; target?: SemanticTarget | null }
  | { kind: "text"; text: string; delayMs: number; target?: SemanticTarget | null }
  | { kind: "key"; keyCode: number; modifiers: number; down: boolean; delayMs: number }
  | { kind: "wait"; durationMs: number }
  | { kind: "system"; command: SystemCommand; delayMs: number }
);

export type MacroDocument = { schemaVersion: number; id: string; name: string; createdAt: string; modifiedAt: string; favorite: boolean; hotkey?: string | null; actions: Action[] };
export type PermissionState = { accessibility: boolean; eventPosting: boolean; inputMonitoring: boolean; screenRecording: boolean; inputMonitoringNote: string };
export type RecordingSettings = { mouseClicks: boolean; mouseMovement: boolean; keyboard: boolean; timing: boolean; scrolling: boolean };
export type AppSettings = { recording: RecordingSettings; pointerHz: number };

export const defaultSettings: RecordingSettings = { mouseClicks: true, mouseMovement: false, keyboard: true, timing: true, scrolling: true };
export const defaultAppSettings: AppSettings = { recording: defaultSettings, pointerHz: 120 };
