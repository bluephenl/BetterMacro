import { type DragEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save as chooseSavePath } from "@tauri-apps/plugin-dialog";
import {
  Archive, ChevronDown, Circle, CircleStop, Clock3, Copy, CornerDownLeft,
  Eye, FileInput, FileOutput, Keyboard, LayoutGrid, ListFilter, MonitorUp, MousePointer2,
  MoreHorizontal, PanelBottom, Play, Plus, Save, Search, Settings2, SlidersHorizontal,
  Star, Trash2, X,
} from "lucide-react";
import {
  Action, AppSettings, defaultAppSettings, MacroDocument, MouseButton, PermissionState,
  RecordingSettings, SemanticTarget, SystemCommand,
} from "./types";
import { actionDelay, actionDuration, normalizeAppSettings, reorder } from "./action-utils";
import appMark from "../BetterMacro logo.png";

type LibraryView = "all" | "recent" | "favorites";
type CommandItem = { name: string; shortcut: string; run: () => void };
type MacroLibrary = { documents: MacroDocument[]; warnings: string[] };
type RuntimeStatus = { emergencyStopAvailable: boolean };

const now = () => new Date().toISOString();
const newMacro = (name = "Untitled macro"): MacroDocument => ({
  schemaVersion: 1, id: `macro-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
  name, createdAt: now(), modifiedAt: now(), favorite: false, actions: [],
});
const cloneAction = (action: Action): Action => structuredClone(action);
const isTypingTarget = (target: EventTarget | null) => target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement;

const systemName: Record<SystemCommand, string> = {
  missionControl: "Open Mission Control",
  showDesktop: "Show Desktop",
  revealDock: "Reveal Dock",
  desktopAndDock: "Show Desktop and Dock",
};
const semanticTargetLabel = (target: SemanticTarget) => [
  (target.subrole ?? target.role).replace(/^AX/, ""),
  target.title ?? target.description ?? target.filename ?? target.value,
].filter(Boolean).join(" · ");
const actionLabel = (action: Action) => {
  switch (action.kind) {
    case "click": return `${action.button === "right" ? "Right click" : action.button === "center" ? "Middle click" : action.button === "other" ? "Other click" : action.clicks > 1 ? "Double click" : "Click"}  ${action.target ? semanticTargetLabel(action.target) : `${Math.round(action.x)}, ${Math.round(action.y)}`}`;
    case "move": return `Move to  ${Math.round(action.x)}, ${Math.round(action.y)}`;
    case "drag": return `Drag  ${action.points.length} points`;
    case "scroll": return `Scroll ${action.y < 0 ? "down" : "up"}  ${Math.abs(action.y)} ${action.unit === "pixel" ? "px" : "lines"}`;
    case "text": return `Type  “${action.text.replace(/\n/g, "↵") || "text"}”`;
    case "key": return `${action.down ? "Press" : "Release"}  ${shortcutFor(action.keyCode, action.modifiers)}`;
    case "wait": return `Wait  ${formatTime(action.durationMs)}`;
    case "system": return systemName[action.command];
  }
};
const actionIcon = (action: Action) => action.kind === "click" || action.kind === "move" || action.kind === "drag" ? MousePointer2 : action.kind === "scroll" ? ListFilter : action.kind === "text" || action.kind === "key" ? Keyboard : action.kind === "system" ? MonitorUp : Clock3;
const formatTime = (ms: number) => ms >= 1000 ? `${(ms / 1000).toFixed(ms % 1000 ? 1 : 0)} sec` : `${ms} ms`;
const shortcutFor = (key: number, modifiers: number) => {
  const signs = `${modifiers & (1 << 20) ? "⌘" : ""}${modifiers & (1 << 19) ? "⌥" : ""}${modifiers & (1 << 18) ? "⌃" : ""}${modifiers & (1 << 17) ? "⇧" : ""}`;
  const names: Record<number, string> = { 36: "Return", 48: "Tab", 49: "Space", 51: "Delete", 53: "Esc", 103: "F11", 123: "←", 124: "→", 125: "↓", 126: "↑" };
  const letter: Record<number, string> = { 0:"A",1:"S",2:"D",3:"F",4:"H",5:"G",6:"Z",7:"X",8:"C",9:"V",11:"B",12:"Q",13:"W",14:"E",15:"R",16:"Y",17:"T",31:"O",32:"U",34:"I",35:"P",37:"L",38:"J",40:"K",45:"N",46:"M" };
  return signs + (names[key] ?? letter[key] ?? `Key ${key}`);
};
const errorMessage = (error: unknown) => typeof error === "string" ? error : error instanceof Error ? error.message : "BetterMacro could not complete that action.";

export function App() {
  const [macros, setMacros] = useState<MacroDocument[]>([]);
  const [active, setActive] = useState<MacroDocument>(() => newMacro());
  const [selected, setSelected] = useState<number | null>(null);
  const [permissions, setPermissions] = useState<PermissionState | null>(null);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [recording, setRecording] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [playIndex, setPlayIndex] = useState<number | null>(null);
  const [speed, setSpeed] = useState(1);
  const [appSettings, setAppSettings] = useState<AppSettings>(() => {
    try {
      return normalizeAppSettings(JSON.parse(localStorage.getItem("bettermacro.settings") ?? "{}"));
    }
    catch { return defaultAppSettings; }
  });
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [palette, setPalette] = useState(false);
  const [permissionOpen, setPermissionOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [macroMenu, setMacroMenu] = useState(false);
  const [insertMenu, setInsertMenu] = useState(false);
  const [libraryView, setLibraryView] = useState<LibraryView>("all");
  const [saveState, setSaveState] = useState("Not saved yet");
  const [notice, setNotice] = useState<string | null>(null);
  const clipboard = useRef<Action | null>(null);
  const undoStack = useRef<MacroDocument[]>([]);
  const redoStack = useRef<MacroDocument[]>([]);
  const suppressStopClick = useRef(false);
  const activeRef = useRef(active);
  const libraryRequested = useRef(false);
  const userChangedActive = useRef(false);
  activeRef.current = active;

  const refreshPermissions = useCallback(async () => { try { setPermissions(await invoke<PermissionState>("permission_status")); } catch { /* browser preview */ } }, []);
  const commit = useCallback((change: (current: MacroDocument) => MacroDocument) => {
    userChangedActive.current = true;
    setActive(current => {
      const next = change(current);
      if (next === current) return current;
      undoStack.current.push(structuredClone(current));
      if (undoStack.current.length > 100) undoStack.current.shift();
      redoStack.current = [];
      setSaveState("Unsaved changes");
      return { ...next, modifiedAt: now() };
    });
  }, []);
  useEffect(() => {
    void refreshPermissions();
    void invoke<RuntimeStatus>("runtime_status").then(setRuntimeStatus).catch(() => null);
    if (libraryRequested.current) return;
    libraryRequested.current = true;
    invoke<MacroLibrary>("load_macros").then(result => {
      setMacros(result.documents);
      if (result.documents[0] && !userChangedActive.current) {
        setActive(result.documents[0]);
        setSaveState("Saved locally");
      }
      if (result.warnings.length) setNotice(result.warnings.join(" "));
    }).catch(error => setNotice(errorMessage(error)));
  }, [refreshPermissions]);
  useEffect(() => { try { localStorage.setItem("bettermacro.settings", JSON.stringify(appSettings)); } catch { /* settings remain active for this session */ } }, [appSettings]);
  useEffect(() => {
    const cleanups = Promise.all([
      listen<Action[]>("recording-actions", event => commit(current => ({ ...current, actions: event.payload, modifiedAt: now() }))),
      listen<boolean>("recording-state", event => setRecording(event.payload)),
      listen<boolean>("playback-state", event => { setPlaying(event.payload); if (!event.payload) setPlayIndex(null); }),
      listen<{ index: number }>("playback-progress", event => setPlayIndex(event.payload.index)),
      listen<string>("playback-error", event => setNotice(event.payload)),
    ]);
    return () => { void cleanups.then(items => items.forEach(unlisten => unlisten())); };
  }, [commit]);

  const undo = useCallback(() => { const previous = undoStack.current.pop(); if (!previous) return; userChangedActive.current = true; setActive(current => { redoStack.current.push(structuredClone(current)); return previous; }); setSaveState("Unsaved changes"); setSelected(null); }, []);
  const redo = useCallback(() => { const next = redoStack.current.pop(); if (!next) return; userChangedActive.current = true; setActive(current => { undoStack.current.push(structuredClone(current)); return next; }); setSaveState("Unsaved changes"); setSelected(null); }, []);

  const visibleActions = useMemo(() => active.actions.map((action, index) => ({ action, index })).filter(({ action }) => actionLabel(action).toLowerCase().includes(search.toLowerCase())), [active.actions, search]);
  const visibleMacros = useMemo(() => libraryView === "favorites" ? macros.filter(item => item.favorite) : libraryView === "recent" ? macros.slice(0, 8) : macros, [libraryView, macros]);
  const currentAction = selected === null ? null : active.actions[selected];

  const updateAction = (index: number, next: Action) => commit(current => ({ ...current, actions: current.actions.map((item, i) => i === index ? next : item), modifiedAt: now() }));
  const removeAction = (index: number) => { commit(current => ({ ...current, actions: current.actions.filter((_, i) => i !== index), modifiedAt: now() })); setSelected(null); };
  const moveAction = (index: number, direction: -1 | 1) => commit(current => { const destination = index + direction; if (destination < 0 || destination >= current.actions.length) return current; setSelected(destination); return { ...current, actions: reorder(current.actions, index, destination), modifiedAt: now() }; });
  const reorderAction = (from: number, to: number) => { if (from === to) return; commit(current => ({ ...current, actions: reorder(current.actions, from, to), modifiedAt: now() })); setSelected(to); };
  const insertAction = (action: Action) => { commit(current => { const at = selected === null ? current.actions.length : selected + 1; setSelected(at); return { ...current, actions: [...current.actions.slice(0, at), action, ...current.actions.slice(at)], modifiedAt: now() }; }); setInsertMenu(false); };
  const duplicateSelected = () => { if (selected === null) return; insertAction(cloneAction(active.actions[selected])); };
  const save = useCallback(async () => { try { const source = active; const document = { ...source, name: source.name.trim() || "Untitled macro", modifiedAt: now() }; await invoke("save_macro", { document }); setMacros(items => [document, ...items.filter(item => item.id !== document.id)]); if (activeRef.current === source) { userChangedActive.current = false; setActive(document); setSaveState("Saved just now"); } } catch (error) { setNotice(errorMessage(error)); } }, [active]);
  const toggleRecording = useCallback(async () => { try { setNotice(null); if (recording) { await invoke("stop_recording"); return; } if (!permissions?.inputMonitoring || !permissions.accessibility) { setPermissionOpen(true); return; } setSelected(null); await invoke("start_recording", { settings: appSettings.recording }); } catch (error) { setNotice(errorMessage(error)); } }, [appSettings.recording, permissions?.accessibility, permissions?.inputMonitoring, recording]);
  const play = useCallback(async (from = 0) => { if (!active.actions.length || playing || recording) return; if (!permissions?.accessibility || !permissions.eventPosting) { setPermissionOpen(true); return; } try { setNotice(null); await invoke("playback", { actions: active.actions, startAt: from, speed, pointerHz: appSettings.pointerHz }); } catch (error) { setNotice(errorMessage(error)); } }, [active.actions, appSettings.pointerHz, permissions, playing, recording, speed]);
  const stop = async () => { try { await invoke(recording ? "stop_recording" : "stop_playback"); } catch (error) { setNotice(errorMessage(error)); } };
  const activateMacro = (macro: MacroDocument, status = "Saved locally") => { userChangedActive.current = true; setActive(macro); setSelected(null); undoStack.current = []; redoStack.current = []; setSaveState(status); };
  const canDiscardChanges = useCallback(() => saveState !== "Unsaved changes" || confirm("Discard the unsaved changes to this macro?"), [saveState]);
  const openMacro = (macro: MacroDocument) => { if (macro.id !== active.id && canDiscardChanges()) activateMacro(macro); };
  const create = useCallback(() => { if (!canDiscardChanges()) return; activateMacro(newMacro(), "Not saved yet"); }, [canDiscardChanges]);
  const duplicateMacro = async () => { const copy = { ...structuredClone(active), id: newMacro().id, name: `${active.name} copy`, createdAt: now(), modifiedAt: now() }; try { await invoke("save_macro", { document: copy }); setMacros(items => [copy, ...items]); activateMacro(copy); setMacroMenu(false); } catch (error) { setNotice(errorMessage(error)); } };
  const deleteMacro = async () => { if (!confirm(`Delete “${active.name}”? This cannot be undone.`)) return; try { if (macros.some(item => item.id === active.id)) await invoke("delete_macro", { id: active.id }); const remaining = macros.filter(item => item.id !== active.id); setMacros(remaining); if (remaining[0]) activateMacro(remaining[0]); else activateMacro(newMacro(), "Not saved yet"); setMacroMenu(false); } catch (error) { setNotice(errorMessage(error)); } };
  const exportMacro = async () => { try { const path = await chooseSavePath({ defaultPath: `${active.name.replace(/[^a-z0-9-_ ]/gi, "").trim() || "macro"}.macro.json`, filters: [{ name: "BetterMacro macro", extensions: ["json"] }] }); if (!path) return; await invoke("export_macro", { document: active, path }); setNotice("Macro exported."); setMacroMenu(false); } catch (error) { setNotice(errorMessage(error)); } };
  const importMacro = useCallback(async () => { if (!canDiscardChanges()) return; try { const path = await open({ multiple: false, directory: false, filters: [{ name: "BetterMacro macro", extensions: ["json", "macro"] }] }); if (typeof path !== "string") return; const imported = await invoke<MacroDocument>("import_macro", { path }); const document = { ...imported, id: newMacro().id, name: `${imported.name.trim() || "Untitled macro"} imported`, createdAt: now(), modifiedAt: now() }; await invoke("save_macro", { document }); setMacros(items => [document, ...items]); activateMacro(document); } catch (error) { setNotice(errorMessage(error)); } }, [canDiscardChanges]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (event.key === "Escape") { setPalette(false); setMacroMenu(false); setInsertMenu(false); return; }
      if (event.metaKey && key === "k") { event.preventDefault(); setPalette(true); return; }
      if (isTypingTarget(event.target)) return;
      if (event.metaKey && key === "s") { event.preventDefault(); void save(); }
      else if (event.metaKey && key === "n") { event.preventDefault(); create(); }
      else if (event.metaKey && key === "o") { event.preventDefault(); void importMacro(); }
      else if (event.metaKey && event.shiftKey && key === "r") { event.preventDefault(); void toggleRecording(); }
      else if (event.metaKey && !event.shiftKey && key === "z") { event.preventDefault(); undo(); }
      else if (event.metaKey && event.shiftKey && key === "z") { event.preventDefault(); redo(); }
      else if (event.metaKey && key === "c" && selected !== null) { event.preventDefault(); clipboard.current = cloneAction(active.actions[selected]); }
      else if (event.metaKey && key === "v" && clipboard.current) { event.preventDefault(); insertAction(cloneAction(clipboard.current)); }
      else if (event.metaKey && key === "d" && selected !== null) { event.preventDefault(); duplicateSelected(); }
      else if ((event.key === "Backspace" || event.key === "Delete") && selected !== null) { event.preventDefault(); removeAction(selected); }
      else if (event.key === " " && !recording && !playing) { event.preventDefault(); void play(); }
    };
    window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler);
  }, [active.actions, create, importMacro, play, playing, recording, redo, save, selected, toggleRecording, undo]);

  const paletteActions: CommandItem[] = [
    { name: recording ? "Stop recording" : "Start recording", shortcut: "⌘⇧R", run: () => void toggleRecording() },
    { name: "Play macro", shortcut: "Space", run: () => void play() },
    { name: "Stop playback", shortcut: runtimeStatus?.emergencyStopAvailable === false ? "" : "⌃⌥Esc", run: () => void invoke("stop_playback") },
    { name: "Insert wait", shortcut: "", run: () => insertAction({ kind: "wait", durationMs: 1000 }) },
    { name: "Open Mission Control", shortcut: "", run: () => insertAction({ kind: "system", command: "missionControl", delayMs: 0 }) },
    { name: "Show Desktop", shortcut: "", run: () => insertAction({ kind: "system", command: "showDesktop", delayMs: 0 }) },
    { name: "Reveal Dock", shortcut: "", run: () => insertAction({ kind: "system", command: "revealDock", delayMs: 0 }) },
    { name: "Show Desktop and Dock", shortcut: "", run: () => insertAction({ kind: "system", command: "desktopAndDock", delayMs: 0 }) },
    { name: "Save macro", shortcut: "⌘S", run: () => void save() },
    { name: "Import macro", shortcut: "⌘O", run: () => void importMacro() },
    { name: "New macro", shortcut: "⌘N", run: create },
    { name: "Recording settings", shortcut: "", run: () => setSettingsOpen(true) },
    { name: "Permissions", shortcut: "", run: () => setPermissionOpen(true) },
  ];

  return <main className="app-shell">
    <header className="topbar">
      <div className="brand"><span className="brand-mark"><img src={appMark} alt=""/></span><span>BetterMacro</span></div>
      <div className="toolbar">
        <button className={`record-button ${recording ? "is-recording" : ""}`} disabled={playing} onClick={() => void toggleRecording()}><Circle size={13} fill="currentColor"/>{recording ? "Recording" : "Record"}</button>
        <button className="tool-button stop" disabled={!recording && !playing} onMouseDown={() => { if (recording) { suppressStopClick.current = true; void stop(); } }} onClick={() => { if (suppressStopClick.current) { suppressStopClick.current = false; return; } void stop(); }}><CircleStop size={16}/> Stop</button>
        <span className="toolbar-divider"/>
        <button className="tool-button" disabled={recording || playing || !active.actions.length} onClick={() => void play()}><Play size={16} fill="currentColor"/> Play</button>
        <label className="speed"><span>Speed</span><select value={speed} onChange={event => setSpeed(Number(event.target.value))}>{[0.25,0.5,1,1.5,2,5].map(option => <option key={option} value={option}>{option}×</option>)}</select></label>
        <span className="toolbar-spacer"/>
        <button className="tool-button" onClick={() => void save()}><Save size={15}/> Save <kbd>⌘S</kbd></button>
      </div>
    </header>

    <aside className="sidebar">
      <div className="sidebar-heading"><span>MACROS</span><div><button aria-label="Import macro" title="Import macro" onClick={() => void importMacro()}><FileInput size={15}/></button><button aria-label="New macro" title="New macro" onClick={create}><Plus size={16}/></button></div></div>
      <button className={`nav-item ${libraryView === "all" ? "active" : ""}`} onClick={() => setLibraryView("all")}><Archive size={15}/> All macros <span>{macros.length}</span></button>
      <button className={`nav-item ${libraryView === "recent" ? "active" : ""}`} onClick={() => setLibraryView("recent")}><Clock3 size={15}/> Recent</button>
      <button className={`nav-item ${libraryView === "favorites" ? "active" : ""}`} onClick={() => setLibraryView("favorites")}><Star size={15}/> Favorites</button>
      <div className="macro-list">
        {visibleMacros.map(macro => <button className={`macro-item ${macro.id === active.id ? "selected" : ""}`} onClick={() => openMacro(macro)} key={macro.id}><span className="macro-dot"/><span>{macro.name}</span>{macro.favorite && <Star size={12} fill="currentColor"/>}</button>)}
        {visibleMacros.length === 0 && <span className="sidebar-empty">No {libraryView === "favorites" ? "favorites" : "saved macros"}</span>}
      </div>
      <div className="sidebar-bottom"><button onClick={() => setPermissionOpen(true)}><Eye size={15}/> Permissions</button><button onClick={() => setSettingsOpen(true)}><Settings2 size={15}/> Settings</button></div>
    </aside>

    <section className="editor">
      <div className="editor-titlebar"><input value={active.name} aria-label="Macro name" onChange={event => commit(current => ({ ...current, name: event.target.value }))}/><button aria-label="Favorite macro" className={active.favorite ? "favorited" : ""} onClick={() => commit(current => ({ ...current, favorite: !current.favorite }))}><Star size={16} fill={active.favorite ? "currentColor" : "none"}/></button><div className="menu-anchor"><button aria-label="Macro actions" onClick={() => setMacroMenu(value => !value)}><MoreHorizontal size={17}/></button>{macroMenu && <MacroMenu duplicate={() => void duplicateMacro()} exportFile={() => void exportMacro()} remove={() => void deleteMacro()} close={() => setMacroMenu(false)}/>}</div></div>
      <div className="editor-subbar"><span>{active.actions.length} actions</span><span>•</span><span>{active.actions.reduce((total, item) => total + actionDuration(item), 0) ? formatTime(active.actions.reduce((total, item) => total + actionDuration(item), 0)) : "No duration"}</span><span className="subbar-spacer"/><div className="menu-anchor"><button onClick={() => setInsertMenu(value => !value)}><Plus size={14}/> Add action <ChevronDown size={12}/></button>{insertMenu && <InsertMenu insert={insertAction}/>}</div><button onClick={() => setSearchOpen(value => !value)}><Search size={14}/> Search</button></div>
      {searchOpen && <div className="action-search"><Search size={15}/><input autoFocus value={search} onChange={event => setSearch(event.target.value)} placeholder="Search actions"/><button onClick={() => { setSearch(""); setSearchOpen(false); }}><X size={15}/></button></div>}
      <div className="timeline">
        {active.actions.length === 0 ? <EmptyState recording={recording} onRecord={() => void toggleRecording()} onCreate={() => insertAction({ kind: "wait", durationMs: 1000 })}/> : visibleActions.map(({ action, index }) => <ActionRow key={index} action={action} index={index} selected={selected === index} executing={playIndex === index} onSelect={() => setSelected(index)} onToggle={() => updateAction(index, { ...action, enabled: action.enabled === false })} onReorder={reorderAction}/>) }
        {active.actions.length > 0 && visibleActions.length === 0 && <div className="no-results">No actions match “{search}”.</div>}
      </div>
      <footer className="statusbar"><span className={recording ? "status-recording" : ""}><i/>{recording ? "Recording globally" : playing ? `Running action ${(playIndex ?? 0) + 1} / ${active.actions.length}` : saveState}</span><span>{recording ? "Press Stop when finished" : playing ? runtimeStatus?.emergencyStopAvailable === false ? "Use Stop to end playback" : "Press ⌃⌥Esc to stop immediately" : runtimeStatus?.emergencyStopAvailable === false ? "Emergency shortcut unavailable · Stop still works" : `${appSettings.pointerHz} pointer events/sec`}</span></footer>
    </section>

    <aside className="inspector">
      <div className="inspector-heading">INSPECTOR</div>
      {currentAction ? <ActionInspector action={currentAction} index={selected!} onChange={updateAction} onDelete={() => removeAction(selected!)} onDuplicate={duplicateSelected} onMove={moveAction} onRun={() => void play(selected!)}/> : <div className="inspector-empty"><SlidersHorizontal size={23}/><p>Select an action to edit its details.</p></div>}
    </aside>

    {permissionOpen && <PermissionDialog state={permissions} refresh={refreshPermissions} reportError={error => setNotice(errorMessage(error))} close={() => setPermissionOpen(false)}/>} 
    {settingsOpen && <SettingsDialog settings={appSettings} change={setAppSettings} close={() => setSettingsOpen(false)}/>} 
    {palette && <CommandPalette close={() => setPalette(false)} actions={paletteActions}/>} 
    {notice && <div className={`error-toast ${notice.endsWith("exported.") ? "success-toast" : ""}`} role="alert"><span>{notice}</span><button onClick={() => setNotice(null)} aria-label="Dismiss"><X size={14}/></button></div>}
  </main>;
}

function ActionRow({ action, index, selected, executing, onSelect, onToggle, onReorder }: { action: Action; index: number; selected: boolean; executing: boolean; onSelect: () => void; onToggle: () => void; onReorder: (from: number, to: number) => void }) {
  const Icon = actionIcon(action); const delay = actionDelay(action);
  const drop = (event: DragEvent) => { event.preventDefault(); const from = Number(event.dataTransfer.getData("text/action-index")); if (Number.isInteger(from)) onReorder(from, index); };
  return <div role="button" tabIndex={0} draggable className={`action-row ${action.enabled === false ? "disabled-action" : ""} ${selected ? "selected" : ""} ${executing ? "executing" : ""}`} onDragStart={event => { event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/action-index", String(index)); }} onDragOver={event => event.preventDefault()} onDrop={drop} onClick={onSelect} onKeyDown={event => { if (event.key === "Enter") onSelect(); }}><span className="action-number">{String(index + 1).padStart(2, "0")}</span><span className="action-icon"><Icon size={15}/></span><span className="action-label">{actionLabel(action)}</span>{delay > 0 && <span className="action-delay">+{formatTime(delay)}</span>}<button className="action-enabled" title={action.enabled === false ? "Enable action" : "Disable action"} aria-label={action.enabled === false ? "Enable action" : "Disable action"} onClick={event => { event.stopPropagation(); onToggle(); }}><Circle size={11} fill="currentColor"/></button></div>;
}
function EmptyState({ recording, onRecord, onCreate }: { recording: boolean; onRecord: () => void; onCreate: () => void }) { return <div className="empty-state"><span className="empty-icon"><img src={appMark} alt=""/></span><h1>{recording ? "Recording your actions" : "No actions yet"}</h1><p>{recording ? "Work in any app. BetterMacro is listening." : "Record anything you repeat."}</p>{!recording && <><button className="primary" onClick={onRecord}><Circle size={13} fill="currentColor"/> Start recording <kbd>⌘⇧R</kbd></button><button className="quiet" onClick={onCreate}>Create manually</button></>}</div>; }

function ActionInspector({ action, index, onChange, onDelete, onDuplicate, onMove, onRun }: { action: Action; index: number; onChange: (index: number, action: Action) => void; onDelete: () => void; onDuplicate: () => void; onMove: (index: number, direction: -1 | 1) => void; onRun: () => void }) {
  const number = (label: string, value: number, change: (value: number) => void, step = 1, disabled = false) => <label className="field"><span>{label}</span><input disabled={disabled} type="number" step={step} value={value} onChange={event => change(Number(event.target.value))}/></label>;
  const title = action.kind === "text" ? "Type text" : action.kind === "key" ? "Key event" : action.kind === "wait" ? "Wait" : action.kind === "scroll" ? "Scroll" : action.kind === "move" ? "Mouse movement" : action.kind === "drag" ? "Mouse drag" : action.kind === "system" ? "macOS action" : "Mouse click";
  const dragStart = action.kind === "drag" ? action.points[0] : null; const dragEnd = action.kind === "drag" ? action.points[action.points.length - 1] : null;
  return <div className="inspector-body">
    <div className="selected-action"><span>{String(index + 1).padStart(2, "0")}</span><strong>{title}</strong></div>
    <label className="switch-field"><span>Enabled</span><input type="checkbox" checked={action.enabled !== false} onChange={event => onChange(index, { ...action, enabled: event.target.checked })}/></label>
    {action.kind === "click" && <><div className="field-grid">{number("X position", Math.round(action.x), x => onChange(index, { ...action, x }))}{number("Y position", Math.round(action.y), y => onChange(index, { ...action, y }))}</div><label className="field"><span>Mouse button</span><select value={action.button} onChange={event => onChange(index, { ...action, button: event.target.value as MouseButton })}><option value="left">Left</option><option value="right">Right</option><option value="center">Middle</option></select></label>{number("Click count", action.clicks, clicks => onChange(index, { ...action, clicks: Math.min(3, Math.max(1, clicks)) }))}</>}
    {action.kind === "move" && <><div className="field-grid">{number("X position", Math.round(action.x), x => onChange(index, { ...action, x }))}{number("Y position", Math.round(action.y), y => onChange(index, { ...action, y }))}</div>{number("Move duration", action.durationMs, durationMs => onChange(index, { ...action, durationMs: Math.max(0, durationMs) }), 10)}</>}
    {action.kind === "drag" && <><div className="field-grid">{number("Start X", Math.round(dragStart?.x ?? 0), () => {}, 1, true)}{number("Start Y", Math.round(dragStart?.y ?? 0), () => {}, 1, true)}{number("End X", Math.round(dragEnd?.x ?? 0), x => { const points = [...action.points]; if (points.length) points[points.length - 1] = { ...points[points.length - 1], x }; onChange(index, { ...action, points }); })}{number("End Y", Math.round(dragEnd?.y ?? 0), y => { const points = [...action.points]; if (points.length) points[points.length - 1] = { ...points[points.length - 1], y }; onChange(index, { ...action, points }); })}</div>{number("Duration", action.durationMs, durationMs => onChange(index, { ...action, durationMs: Math.max(1, durationMs) }), 10)}<span className="field-note">{action.points.length} recorded path points</span></>}
    {action.kind === "scroll" && <><div className="field-grid">{number("Horizontal", action.x, x => onChange(index, { ...action, x }))}{number("Vertical", action.y, y => onChange(index, { ...action, y }))}</div><label className="field"><span>Unit</span><select value={action.unit ?? "line"} onChange={event => onChange(index, { ...action, unit: event.target.value as "pixel" | "line" })}><option value="line">Lines</option><option value="pixel">Pixels</option></select></label></>}
    {action.kind === "text" && <><label className="field"><span>Text</span><textarea value={action.text} onChange={event => onChange(index, { ...action, text: event.target.value })}/></label>{action.target && <div className="inspector-section"><span>Targeting</span><div className="target-option selected"><i/><span>Semantic text field</span></div><span className="field-note">{semanticTargetLabel(action.target)}</span></div>}</>}
    {action.kind === "key" && <><label className="field"><span>Key code</span><input type="number" value={action.keyCode} onChange={event => onChange(index, { ...action, keyCode: Number(event.target.value) })}/></label><label className="switch-field"><span>Key down</span><input type="checkbox" checked={action.down} onChange={event => onChange(index, { ...action, down: event.target.checked })}/></label></>}
    {action.kind === "system" && <label className="field"><span>macOS action</span><select value={action.command} onChange={event => onChange(index, { ...action, command: event.target.value as SystemCommand })}><option value="missionControl">Mission Control</option><option value="showDesktop">Show Desktop</option><option value="revealDock">Reveal Dock</option><option value="desktopAndDock">Show Desktop and Dock</option></select></label>}
    {action.kind === "wait" ? number("Duration", action.durationMs, durationMs => onChange(index, { ...action, durationMs: Math.max(0, durationMs) }), 50) : number("Wait before", action.delayMs, delayMs => onChange(index, { ...action, delayMs: Math.max(0, delayMs) }), 50)}
    {(action.kind === "click" || action.kind === "scroll" || action.kind === "move" || action.kind === "drag") && <div className="inspector-section"><span>Targeting</span>{(action.kind === "click" || action.kind === "scroll") && action.target ? <><div className="target-option selected"><i/><span>Semantic UI element</span></div><span className="field-note">{semanticTargetLabel(action.target)}</span><div className="target-option"><i/><span>Screen coordinates</span><em>Used only for macros without semantic data</em></div></> : <div className="target-option selected"><i/><span>Screen coordinates</span></div>}<div className="target-option muted"><i/><span>Image target</span><em>Not yet available</em></div></div>}
    <div className="inspector-actions"><button onClick={onRun}><Play size={14} fill="currentColor"/> Run from here</button><button onClick={() => onMove(index, -1)}>Move up</button><button onClick={() => onMove(index, 1)}>Move down</button><button onClick={onDuplicate}><Copy size={13}/> Duplicate</button><button className="danger" onClick={onDelete}><Trash2 size={14}/> Delete</button></div>
  </div>;
}

function MacroMenu({ duplicate, exportFile, remove, close }: { duplicate: () => void; exportFile: () => void; remove: () => void; close: () => void }) { return <div className="popover-menu" onMouseLeave={close}><button onClick={duplicate}><Copy size={13}/> Duplicate macro</button><button onClick={exportFile}><FileOutput size={13}/> Export macro</button><span/><button className="danger" onClick={remove}><Trash2 size={13}/> Delete macro</button></div>; }
function InsertMenu({ insert }: { insert: (action: Action) => void }) { return <div className="popover-menu insert-menu"><button onClick={() => insert({ kind: "wait", durationMs: 1000 })}><Clock3 size={13}/> Wait</button><button onClick={() => insert({ kind: "text", text: "", delayMs: 0, target: null })}><Keyboard size={13}/> Type text</button><span/><button onClick={() => insert({ kind: "system", command: "missionControl", delayMs: 0 })}><LayoutGrid size={13}/> Mission Control</button><button onClick={() => insert({ kind: "system", command: "showDesktop", delayMs: 0 })}><MonitorUp size={13}/> Show Desktop</button><button onClick={() => insert({ kind: "system", command: "revealDock", delayMs: 0 })}><PanelBottom size={13}/> Reveal Dock</button><button onClick={() => insert({ kind: "system", command: "desktopAndDock", delayMs: 0 })}><PanelBottom size={13}/> Desktop and Dock</button></div>; }

function SettingsDialog({ settings, change, close }: { settings: AppSettings; change: (settings: AppSettings) => void; close: () => void }) {
  const setRecording = (patch: Partial<RecordingSettings>) => change({ ...settings, recording: { ...settings.recording, ...patch } });
  return <div className="dialog-backdrop" onMouseDown={close}><section className="permission-dialog settings-dialog" role="dialog" aria-modal="true" onMouseDown={event => event.stopPropagation()}><header><div><span className="dialog-kicker">SETTINGS</span><h2>Recording and playback</h2></div><button onClick={close} aria-label="Close"><X size={18}/></button></header><p>These defaults apply immediately and are stored only on this Mac.</p><div className="settings-list"><SettingToggle label="Mouse clicks and drags" checked={settings.recording.mouseClicks} change={mouseClicks => setRecording({ mouseClicks })}/><SettingToggle label="Visible mouse movement rows" detail="Click and drag paths are always retained internally." checked={settings.recording.mouseMovement} change={mouseMovement => setRecording({ mouseMovement })}/><SettingToggle label="Keyboard" checked={settings.recording.keyboard} change={keyboard => setRecording({ keyboard })}/><SettingToggle label="Timing" checked={settings.recording.timing} change={timing => setRecording({ timing })}/><SettingToggle label="Scrolling" checked={settings.recording.scrolling} change={scrolling => setRecording({ scrolling })}/><label className="setting-row"><span><strong>Pointer playback rate</strong><small>Higher rates make long moves and drags smoother.</small></span><select value={settings.pointerHz} onChange={event => change({ ...settings, pointerHz: Number(event.target.value) })}><option value={60}>60/sec</option><option value={120}>120/sec</option><option value={240}>240/sec</option></select></label></div><footer><button className="quiet-inline" onClick={() => change(defaultAppSettings)}>Restore defaults</button><button className="primary" onClick={close}>Done</button></footer></section></div>;
}
function SettingToggle({ label, detail, checked, change }: { label: string; detail?: string; checked: boolean; change: (checked: boolean) => void }) { return <label className="setting-row"><span><strong>{label}</strong>{detail && <small>{detail}</small>}</span><input type="checkbox" checked={checked} onChange={event => change(event.target.checked)}/></label>; }

function PermissionDialog({ state, refresh, reportError, close }: { state: PermissionState | null; refresh: () => Promise<void>; reportError: (error: unknown) => void; close: () => void }) {
  const [requesting, setRequesting] = useState(false);
  useEffect(() => { const timer = window.setInterval(() => void refresh(), 1000); return () => window.clearInterval(timer); }, [refresh]);
  const grant = async (kind: "accessibility" | "input" | "screen") => { setRequesting(true); try { const command = kind === "accessibility" ? "request_accessibility" : kind === "input" ? "request_input_monitoring" : "request_screen_recording"; await invoke(command); await refresh(); } catch (error) { reportError(error); } finally { setRequesting(false); } };
  return <div className="dialog-backdrop"><section className="permission-dialog" role="dialog" aria-modal="true"><header><div><span className="dialog-kicker">MACOS PERMISSIONS</span><h2>Set up BetterMacro</h2></div><button onClick={close} aria-label="Close"><X size={18}/></button></header><p>BetterMacro asks only when a feature needs access. Your recordings stay on this Mac.</p><PermissionRow title="Accessibility" allowed={(state?.accessibility ?? false) && (state?.eventPosting ?? false)} detail="Required to identify UI elements and replay actions reliably." onGrant={() => void grant("accessibility")} loading={requesting}/><PermissionRow title="Input Monitoring" allowed={state?.inputMonitoring ?? false} detail="Required to record keyboard, trackpad, and mouse actions outside BetterMacro." onGrant={() => void grant("input")} loading={requesting}/><PermissionRow title="Screen Recording" allowed={state?.screenRecording ?? false} detail="Only needed when you capture or match an image target." onGrant={() => void grant("screen")} loading={requesting}/><footer><span>{state?.inputMonitoringNote}</span><button className="primary" onClick={close}>Done</button></footer></section></div>;
}
function PermissionRow({ title, allowed, detail, onGrant, loading }: { title: string; allowed: boolean; detail: string; onGrant?: () => void; loading: boolean }) { return <div className="permission-row"><span className={`permission-indicator ${allowed ? "allowed" : ""}`}/><div><strong>{title}</strong><p>{detail}</p><small>{allowed ? "Allowed" : "Permission required"}</small></div>{!allowed && onGrant && <button onClick={onGrant} disabled={loading}>Grant access</button>}</div>; }

function CommandPalette({ close, actions }: { close: () => void; actions: CommandItem[] }) {
  const [query, setQuery] = useState(""); const [highlighted, setHighlighted] = useState(0); const filtered = actions.filter(action => action.name.toLowerCase().includes(query.toLowerCase()));
  useEffect(() => setHighlighted(0), [query]);
  const run = (action?: CommandItem) => { if (!action) return; close(); action.run(); };
  return <div className="dialog-backdrop palette-backdrop" onMouseDown={close}><section className="command-palette" onMouseDown={event => event.stopPropagation()}><div className="palette-input"><Search size={17}/><input autoFocus value={query} onChange={event => setQuery(event.target.value)} onKeyDown={event => { if (event.key === "Escape") close(); else if (event.key === "ArrowDown") { event.preventDefault(); setHighlighted(value => Math.min(filtered.length - 1, value + 1)); } else if (event.key === "ArrowUp") { event.preventDefault(); setHighlighted(value => Math.max(0, value - 1)); } else if (event.key === "Enter") { event.preventDefault(); run(filtered[highlighted]); } }} placeholder="Search commands"/><kbd>Esc</kbd></div><div className="palette-list">{filtered.map((action, index) => <button className={index === highlighted ? "highlighted" : ""} key={action.name} onMouseEnter={() => setHighlighted(index)} onClick={() => run(action)}><span>{action.name}</span><kbd>{action.shortcut}</kbd></button>)}{filtered.length === 0 && <div className="palette-empty">No matching commands</div>}</div><footer><span><CornerDownLeft size={13}/> to run</span><span><span>↑↓</span> to navigate</span></footer></section></div>;
}
