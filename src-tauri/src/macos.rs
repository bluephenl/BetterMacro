#![cfg(target_os = "macos")]

use crate::model::{
    Action, MouseButton, PlaybackProgress, PointerSample, RecordingSettings, RelativePoint,
    ScreenPoint, ScrollUnit, SemanticNode, SemanticTarget, SystemCommand,
};
use serde::Serialize;
use std::{
    collections::{HashSet, VecDeque},
    ffi::c_void,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;
type CFArrayRef = *const c_void;
type AXUIElementRef = *const c_void;
type AXValueRef = *const c_void;
type CGEventMask = u64;
type CGEventType = u32;
type CGEventFlags = u64;

#[repr(C)]
#[derive(Copy, Clone)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DockEdge {
    Bottom,
    Left,
    Right,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGPreflightPostEventAccess() -> bool;
    fn CGRequestPostEventAccess() -> bool;
    fn CGEventTapCreate(
        location: u32,
        placement: u32,
        options: u32,
        events: CGEventMask,
        callback: extern "C" fn(
            CGEventTapProxy,
            CGEventType,
            CGEventRef,
            *mut c_void,
        ) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventCreate(source: *mut c_void) -> CGEventRef;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetDoubleValueField(event: CGEventRef, field: u32) -> f64;
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventKeyboardGetUnicodeString(
        event: CGEventRef,
        max_length: usize,
        actual_length: *mut usize,
        unicode_string: *mut u16,
    );
    fn CGEventCreateMouseEvent(
        source: *mut c_void,
        event_type: CGEventType,
        point: CGPoint,
        button: u32,
    ) -> CGEventRef;
    fn CGEventCreateKeyboardEvent(source: *mut c_void, keycode: u16, key_down: bool) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent2(
        source: *mut c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CGGetDisplaysWithPoint(
        point: CGPoint,
        max_displays: u32,
        displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CFRelease(value: *const c_void);
    fn CFRetain(value: *const c_void) -> *const c_void;
    fn CFEqual(value1: *const c_void, value2: *const c_void) -> bool;
    fn CFGetTypeID(value: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringGetLength(value: CFStringRef) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(
        value: CFStringRef,
        buffer: *mut i8,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        value: *const i8,
        encoding: u32,
    ) -> CFStringRef;
    fn CFArrayGetTypeID() -> usize;
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> *const c_void;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyElementAtPosition(
        application: AXUIElementRef,
        x: f32,
        y: f32,
        element: *mut AXUIElementRef,
    ) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXUIElementCopyActionNames(element: AXUIElementRef, names: *mut CFArrayRef) -> i32;
    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> i32;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetType(value: AXValueRef) -> u32;
    fn AXValueGetValue(value: AXValueRef, value_type: u32, value_ptr: *mut c_void) -> bool;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(loop_ref: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: CFStringRef;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    static kCFBooleanTrue: *const c_void;
}

#[link(name = "proc")]
extern "C" {
    fn proc_pidpath(pid: i32, buffer: *mut c_void, buffer_size: u32) -> i32;
}

const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const MOUSE_MOVED: u32 = 5;
const LEFT_DRAGGED: u32 = 6;
const RIGHT_DRAGGED: u32 = 7;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const SCROLL: u32 = 22;
const OTHER_DOWN: u32 = 25;
const OTHER_UP: u32 = 26;
const OTHER_DRAGGED: u32 = 27;
const HID_TAP: u32 = 0;
const HEAD_INSERT: u32 = 0;
const LISTEN_ONLY: u32 = 1;
const FIELD_MOUSE_CLICK_STATE: u32 = 1;
const FIELD_MOUSE_BUTTON: u32 = 3;
const FIELD_KEYCODE: u32 = 9;
const FIELD_SCROLL_DELTA_Y: u32 = 11;
const FIELD_SCROLL_DELTA_X: u32 = 12;
const FIELD_SCROLL_IS_CONTINUOUS: u32 = 88;
const FIELD_SCROLL_POINT_DELTA_Y: u32 = 96;
const FIELD_SCROLL_POINT_DELTA_X: u32 = 97;
const FIELD_SCROLL_FIXED_DELTA_Y: u32 = 93;
const FIELD_SCROLL_FIXED_DELTA_X: u32 = 94;
const EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = u32::MAX - 1;
const EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = u32::MAX;
const SHIFT: u64 = 1 << 17;
const CONTROL: u64 = 1 << 18;
const OPTION: u64 = 1 << 19;
const COMMAND: u64 = 1 << 20;
const SECONDARY_FN: u64 = 1 << 23;
const KEY_SHIFT: u16 = 56;
const KEY_CONTROL: u16 = 59;
const KEY_OPTION: u16 = 58;
const KEY_COMMAND: u16 = 55;
const KEY_FUNCTION: u16 = 63;
const UTF8_ENCODING: u32 = 0x0800_0100;
const AX_VALUE_CGPOINT: u32 = 1;
const AX_VALUE_CGSIZE: u32 = 2;
const AX_SUCCESS: i32 = 0;

fn open_privacy_pane(anchor: &str) {
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
    let _ = std::process::Command::new("open").arg(url).spawn();
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionState {
    pub accessibility: bool,
    pub event_posting: bool,
    pub input_monitoring: bool,
    pub screen_recording: bool,
    pub input_monitoring_note: String,
}

struct Recording {
    active: bool,
    settings: RecordingSettings,
    actions: Vec<Action>,
    last: Instant,
    last_move: Option<(f64, f64, Instant)>,
    pending_moves: VecDeque<MouseSample>,
    pending_pointer: Option<PendingPointer>,
    grouped_text_keys: HashSet<u16>,
    scroll_remainder_x: f64,
    scroll_remainder_y: f64,
    last_scroll_at: Option<Instant>,
}

#[derive(Clone, Copy)]
struct MouseSample {
    x: f64,
    y: f64,
    at: Instant,
}

struct PendingPointer {
    button: MouseButton,
    clicks: u8,
    started: Instant,
    delay_ms: u64,
    approach: Vec<PointerSample>,
    points: Vec<PointerSample>,
    target: Option<SemanticTarget>,
}
impl Recording {
    fn new() -> Self {
        Self {
            active: false,
            settings: RecordingSettings::default(),
            actions: vec![],
            last: Instant::now(),
            last_move: None,
            pending_moves: VecDeque::new(),
            pending_pointer: None,
            grouped_text_keys: HashSet::new(),
            scroll_remainder_x: 0.0,
            scroll_remainder_y: 0.0,
            last_scroll_at: None,
        }
    }
}

pub struct MacAutomation {
    recording: Mutex<Recording>,
    mode: Mutex<AutomationMode>,
    playing: AtomicBool,
    listener_started: AtomicBool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationMode {
    Idle,
    Recording,
    Playing,
}

impl MacAutomation {
    pub fn new() -> Self {
        Self {
            recording: Mutex::new(Recording::new()),
            mode: Mutex::new(AutomationMode::Idle),
            playing: AtomicBool::new(false),
            listener_started: AtomicBool::new(false),
        }
    }
    pub fn permission_status(&self) -> PermissionState {
        unsafe {
            PermissionState {
                accessibility: AXIsProcessTrusted(),
                event_posting: CGPreflightPostEventAccess(),
                input_monitoring: CGPreflightListenEventAccess(),
                screen_recording: CGPreflightScreenCaptureAccess(),
                input_monitoring_note:
                    "Input Monitoring is required to record actions outside BetterMacro.".into(),
            }
        }
    }
    pub fn request_accessibility(&self) -> bool {
        // Asking with this documented option lets macOS show its native Privacy prompt.
        let trusted = unsafe {
            let key = kAXTrustedCheckOptionPrompt;
            let value = kCFBooleanTrue;
            let options = CFDictionaryCreate(
                std::ptr::null(),
                &key,
                &value,
                1,
                std::ptr::null(),
                std::ptr::null(),
            );
            let trusted = AXIsProcessTrustedWithOptions(options);
            if !options.is_null() {
                CFRelease(options);
            }
            trusted
        };
        let event_posting = unsafe { CGRequestPostEventAccess() };
        if !trusted || !event_posting {
            // macOS only displays the native alert once. Subsequent requests must
            // take the user to the Accessibility privacy list.
            open_privacy_pane("Privacy_Accessibility");
        }
        trusted && event_posting
    }
    pub fn request_screen_recording(&self) -> bool {
        let allowed = unsafe { CGRequestScreenCaptureAccess() };
        if !allowed {
            // Screen Recording also stops showing its prompt after it has been
            // answered once, so keep the button useful on every later attempt.
            open_privacy_pane("Privacy_ScreenCapture");
        }
        allowed
    }
    pub fn request_input_monitoring(&self) -> bool {
        let allowed = unsafe { CGRequestListenEventAccess() };
        if !allowed {
            // Once a user has dismissed the native prompt, macOS expects them to
            // change this permission in System Settings (System Preferences on 12).
            open_privacy_pane("Privacy_ListenEvent");
        }
        allowed
    }
    pub fn start_recording(&self, settings: RecordingSettings) -> Result<(), String> {
        let permissions = self.permission_status();
        if !permissions.input_monitoring {
            return Err(
                "Input Monitoring permission is required before recording outside BetterMacro."
                    .into(),
            );
        }
        if !permissions.accessibility {
            return Err(
                "Accessibility permission is required to identify UI elements while recording."
                    .into(),
            );
        }
        let mut mode = self.mode.lock().map_err(|_| "Automation state is busy")?;
        if *mode != AutomationMode::Idle {
            return Err(
                "Stop the current recording or playback before starting a recording.".into(),
            );
        }
        let mut recording = self.recording.lock().map_err(|_| "Recorder is busy")?;
        recording.active = true;
        recording.settings = settings;
        recording.actions.clear();
        recording.last = Instant::now();
        recording.last_move = None;
        recording.pending_moves.clear();
        recording.pending_pointer = None;
        recording.grouped_text_keys.clear();
        recording.scroll_remainder_x = 0.0;
        recording.scroll_remainder_y = 0.0;
        recording.last_scroll_at = None;
        *mode = AutomationMode::Recording;
        Ok(())
    }
    pub fn stop_recording(&self) -> Result<Vec<Action>, String> {
        let mut mode = self.mode.lock().map_err(|_| "Automation state is busy")?;
        let mut r = self.recording.lock().map_err(|_| "Recorder is busy")?;
        r.active = false;
        if *mode == AutomationMode::Recording {
            *mode = AutomationMode::Idle;
        }
        Ok(r.actions.clone())
    }
    pub fn stop_playback(&self) {
        self.playing.store(false, Ordering::SeqCst);
    }
    pub fn begin_playback(&self) -> Result<(), String> {
        let mut mode = self.mode.lock().map_err(|_| "Automation state is busy")?;
        if *mode != AutomationMode::Idle {
            return Err("Stop the current recording or playback before starting playback.".into());
        }
        *mode = AutomationMode::Playing;
        self.playing.store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn complete_playback(&self) {
        self.playing.store(false, Ordering::SeqCst);
        if let Ok(mut mode) = self.mode.lock() {
            if *mode == AutomationMode::Playing {
                *mode = AutomationMode::Idle;
            }
        }
    }

    pub fn ensure_event_listener(self: &Arc<Self>) -> Result<(), String> {
        if self
            .listener_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }
        let automation = Arc::clone(self);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || unsafe {
            LISTENER.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&automation)));
            let mask = [
                LEFT_DOWN,
                LEFT_UP,
                RIGHT_DOWN,
                RIGHT_UP,
                OTHER_DOWN,
                OTHER_UP,
                MOUSE_MOVED,
                LEFT_DRAGGED,
                RIGHT_DRAGGED,
                OTHER_DRAGGED,
                KEY_DOWN,
                KEY_UP,
                SCROLL,
            ]
            .iter()
            .fold(0u64, |mask, event| mask | (1u64 << *event));
            let tap = CGEventTapCreate(
                HID_TAP,
                HEAD_INSERT,
                LISTEN_ONLY,
                mask,
                event_callback,
                std::ptr::null_mut(),
            );
            if tap.is_null() {
                automation.listener_started.store(false, Ordering::SeqCst);
                let _ = ready_tx.send(Err(
                    "macOS blocked global input recording. Enable BetterMacro in Privacy & Security → Input Monitoring, then try Record again."
                        .to_string(),
                ));
                return;
            }
            EVENT_TAP.with(|slot| slot.set(tap));
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                CFRelease(tap);
                automation.listener_started.store(false, Ordering::SeqCst);
                let _ = ready_tx.send(Err("Unable to start the macOS input listener.".to_string()));
                return;
            }
            let runloop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(runloop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            let _ = ready_tx.send(Ok(()));
            CFRunLoopRun();
            CFRelease(source);
            CFRelease(tap);
            EVENT_TAP.with(|slot| slot.set(std::ptr::null_mut()));
            automation.listener_started.store(false, Ordering::SeqCst);
        });
        ready_rx
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "The macOS input listener did not start in time.".to_string())?
    }

    fn record_event(&self, event_type: u32, event: CGEventRef) {
        let mut r = match self.recording.lock() {
            Ok(value) => value,
            Err(_) => return,
        };
        if !r.active {
            return;
        }
        let now = Instant::now();
        let delay = if r.settings.timing {
            now.duration_since(r.last).as_millis() as u64
        } else {
            0
        };
        let point = unsafe { CGEventGetLocation(event) };
        match event_type {
            LEFT_DOWN | RIGHT_DOWN | OTHER_DOWN if r.settings.mouse_clicks => {
                let button = mouse_button_for_event(event);
                let clicks = unsafe { CGEventGetIntegerValueField(event, FIELD_MOUSE_CLICK_STATE) }
                    .clamp(1, 3) as u8;
                let approach = if r.settings.mouse_movement {
                    r.pending_moves.clear();
                    Vec::new()
                } else {
                    let previous_action_at = r.last;
                    r.pending_moves
                        .drain(..)
                        .filter(|sample| sample.at >= previous_action_at)
                        .map(|sample| PointerSample {
                            x: sample.x,
                            y: sample.y,
                            offset_ms: sample
                                .at
                                .duration_since(previous_action_at)
                                .as_millis()
                                .min(delay as u128) as u64,
                            screen_edge: point_touches_display_edge(sample.x, sample.y),
                        })
                        .collect()
                };
                r.pending_pointer = Some(PendingPointer {
                    button,
                    clicks,
                    delay_ms: delay,
                    started: now,
                    approach,
                    target: semantic_target_at(point.x, point.y),
                    points: vec![PointerSample {
                        x: point.x,
                        y: point.y,
                        offset_ms: 0,
                        screen_edge: point_touches_display_edge(point.x, point.y),
                    }],
                });
            }
            LEFT_DRAGGED | RIGHT_DRAGGED | OTHER_DRAGGED if r.settings.mouse_clicks => {
                if let Some(pending) = r.pending_pointer.as_mut() {
                    let offset_ms = now.duration_since(pending.started).as_millis() as u64;
                    let should_keep = pending
                        .points
                        .last()
                        .map(|sample| {
                            (sample.x - point.x).hypot(sample.y - point.y) >= 1.0
                                || offset_ms.saturating_sub(sample.offset_ms) >= 8
                        })
                        .unwrap_or(true);
                    if should_keep {
                        pending.points.push(PointerSample {
                            x: point.x,
                            y: point.y,
                            offset_ms,
                            screen_edge: point_touches_display_edge(point.x, point.y),
                        });
                    }
                }
            }
            LEFT_UP | RIGHT_UP | OTHER_UP if r.settings.mouse_clicks => {
                let Some(mut pending) = r.pending_pointer.take() else {
                    return;
                };
                if pending.button != mouse_button_for_event(event) {
                    return;
                }
                let duration_ms = now.duration_since(pending.started).as_millis() as u64;
                let start = &pending.points[0];
                let traveled = (start.x - point.x).hypot(start.y - point.y);
                let last_is_final = pending
                    .points
                    .last()
                    .is_some_and(|sample| (sample.x - point.x).hypot(sample.y - point.y) < 0.5);
                if !last_is_final {
                    pending.points.push(PointerSample {
                        x: point.x,
                        y: point.y,
                        offset_ms: duration_ms,
                        screen_edge: point_touches_display_edge(point.x, point.y),
                    });
                }

                if traveled >= 3.0 || pending.points.len() > 2 {
                    r.actions.push(Action::Drag {
                        enabled: true,
                        button: pending.button,
                        approach: pending.approach,
                        points: pending.points,
                        duration_ms,
                        delay_ms: pending.delay_ms,
                    });
                } else if pending.clicks > 1
                    && merge_multi_click(
                        &mut r.actions,
                        point.x,
                        point.y,
                        pending.button,
                        pending.clicks,
                    )
                {
                    // The first click already owns the approach and delay.
                } else {
                    r.actions.push(Action::Click {
                        enabled: true,
                        x: point.x,
                        y: point.y,
                        button: pending.button,
                        clicks: pending.clicks,
                        delay_ms: pending.delay_ms,
                        approach: pending.approach,
                        target: pending.target,
                    });
                }
                r.last = now;
            }
            MOUSE_MOVED => {
                if r.settings.mouse_movement {
                    let should_keep = r
                        .last_move
                        .map(|(x, y, at)| {
                            (x - point.x).hypot(y - point.y) > 12.0
                                || now.duration_since(at) > Duration::from_millis(180)
                        })
                        .unwrap_or(true);
                    if should_keep {
                        r.actions.push(Action::Move {
                            enabled: true,
                            x: point.x,
                            y: point.y,
                            duration_ms: 0,
                            delay_ms: delay,
                            screen_edge: point_touches_display_edge(point.x, point.y),
                        });
                        r.last_move = Some((point.x, point.y, now));
                        r.last = now;
                    }
                } else {
                    let should_keep = r
                        .pending_moves
                        .back()
                        .map(|sample| {
                            (sample.x - point.x).hypot(sample.y - point.y) > 30.0
                                || now.duration_since(sample.at) >= Duration::from_millis(70)
                        })
                        .unwrap_or(true);
                    if should_keep {
                        r.pending_moves.push_back(MouseSample {
                            x: point.x,
                            y: point.y,
                            at: now,
                        });
                        while r.pending_moves.len() > 128 {
                            r.pending_moves.pop_front();
                        }
                        while r.pending_moves.front().is_some_and(|sample| {
                            now.duration_since(sample.at) > Duration::from_secs(5)
                        }) {
                            r.pending_moves.pop_front();
                        }
                    }
                }
            }
            SCROLL if r.settings.scrolling => {
                let continuous =
                    unsafe { CGEventGetIntegerValueField(event, FIELD_SCROLL_IS_CONTINUOUS) != 0 };
                let (x, y, unit) = if continuous {
                    (
                        unsafe { continuous_scroll_delta(event, false, &mut r.scroll_remainder_x) },
                        unsafe { continuous_scroll_delta(event, true, &mut r.scroll_remainder_y) },
                        ScrollUnit::Pixel,
                    )
                } else {
                    (
                        unsafe { CGEventGetIntegerValueField(event, FIELD_SCROLL_DELTA_X) as i32 },
                        unsafe { CGEventGetIntegerValueField(event, FIELD_SCROLL_DELTA_Y) as i32 },
                        ScrollUnit::Line,
                    )
                };
                if x != 0 || y != 0 {
                    let merges_previous = r
                        .last_scroll_at
                        .is_some_and(|at| now.duration_since(at) <= Duration::from_millis(120));
                    let merged = if merges_previous {
                        match r.actions.last_mut() {
                            Some(Action::Scroll {
                                x: previous_x,
                                y: previous_y,
                                unit: previous_unit,
                                ..
                            }) if *previous_unit == unit => {
                                *previous_x = previous_x.saturating_add(x);
                                *previous_y = previous_y.saturating_add(y);
                                true
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };
                    if !merged {
                        r.actions.push(Action::Scroll {
                            enabled: true,
                            x,
                            y,
                            unit,
                            at: Some(ScreenPoint {
                                x: point.x,
                                y: point.y,
                            }),
                            target: semantic_target_at(point.x, point.y),
                            delay_ms: delay,
                        });
                    }
                    r.last_scroll_at = Some(now);
                    r.last = now;
                }
            }
            KEY_DOWN | KEY_UP if r.settings.keyboard => {
                let key = unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } as u16;
                let modifiers =
                    unsafe { CGEventGetFlags(event) } & (SHIFT | CONTROL | OPTION | COMMAND);
                if event_type == KEY_UP && r.grouped_text_keys.remove(&key) {
                    return;
                }
                if event_type == KEY_DOWN && modifiers & (CONTROL | OPTION | COMMAND) == 0 {
                    let typed = unsafe { unicode_text_for_event(event) };
                    if typed.as_ref().is_some_and(|text| {
                        !text.is_empty() && text.chars().all(|character| !character.is_control())
                    }) {
                        let typed = typed.expect("checked above");
                        r.grouped_text_keys.insert(key);
                        if can_append_text(&r.actions, delay) {
                            if let Some(Action::Text { text, .. }) = r.actions.last_mut() {
                                text.push_str(&typed);
                                r.last = now;
                                return;
                            }
                        }
                        let target = semantic_focused_target();
                        r.actions.push(Action::Text {
                            enabled: true,
                            text: typed,
                            delay_ms: delay,
                            target,
                        });
                        r.last = now;
                        return;
                    }
                }
                r.actions.push(Action::Key {
                    enabled: true,
                    key_code: key,
                    modifiers,
                    down: event_type == KEY_DOWN,
                    delay_ms: delay,
                });
                r.last = now;
            }
            _ => {}
        }
    }

    pub fn play<F: FnMut(PlaybackProgress)>(
        &self,
        actions: &[Action],
        start_at: usize,
        speed: f64,
        pointer_hz: u16,
        mut progress: F,
    ) -> Result<(), String> {
        if !self.playing.load(Ordering::SeqCst) {
            return Ok(());
        }
        let permissions = self.permission_status();
        if !permissions.accessibility || !permissions.event_posting {
            return Err(
                "Accessibility input control is not fully allowed. Open Permissions and grant Accessibility again."
                    .into(),
            );
        }
        let speed = speed.clamp(0.25, 5.0);
        let pointer_hz = pointer_hz.clamp(30, 240);
        let mut pressed_keys = HashSet::new();
        let result = (|| {
            for (offset, action) in actions.iter().skip(start_at).enumerate() {
                if !self.playing.load(Ordering::SeqCst) {
                    return Ok(());
                }
                if !action.enabled() {
                    progress(PlaybackProgress {
                        index: start_at + offset,
                        total: actions.len(),
                    });
                    continue;
                }
                let completed = match action {
                    Action::Click {
                        x,
                        y,
                        button,
                        clicks,
                        delay_ms,
                        approach,
                        target,
                        ..
                    } => self.play_click(
                        *x,
                        *y,
                        *button,
                        *clicks,
                        *delay_ms,
                        approach,
                        target.as_ref(),
                        speed,
                        pointer_hz,
                    )?,
                    Action::Move {
                        x,
                        y,
                        duration_ms,
                        delay_ms,
                        screen_edge,
                        ..
                    } => {
                        let start = unsafe { current_pointer_location()? };
                        let move_duration = if *duration_ms == 0 {
                            *delay_ms
                        } else {
                            if !self.wait_interruptibly(*delay_ms, speed) {
                                return Ok(());
                            }
                            *duration_ms
                        };
                        let moved = self.smooth_pointer(
                            start,
                            CGPoint { x: *x, y: *y },
                            move_duration,
                            speed,
                            pointer_hz,
                            MOUSE_MOVED,
                            0,
                        )?;
                        moved && (!*screen_edge || self.wait_interruptibly(450, 1.0))
                    }
                    Action::Drag {
                        button,
                        approach,
                        points,
                        duration_ms,
                        delay_ms,
                        ..
                    } => self.play_drag(
                        *button,
                        approach,
                        points,
                        *duration_ms,
                        *delay_ms,
                        speed,
                        pointer_hz,
                    )?,
                    Action::System {
                        command, delay_ms, ..
                    } => {
                        if !self.wait_interruptibly(*delay_ms, speed) {
                            false
                        } else {
                            self.play_system_action(*command, pointer_hz)?
                        }
                    }
                    Action::Text {
                        text,
                        delay_ms,
                        target,
                        ..
                    } => {
                        if !self.wait_interruptibly(*delay_ms, speed) {
                            false
                        } else {
                            self.play_text(text, target.as_ref())?
                        }
                    }
                    Action::Scroll {
                        x,
                        y,
                        unit,
                        delay_ms,
                        at,
                        target,
                        ..
                    } => self.play_scroll(
                        *x,
                        *y,
                        *unit,
                        *delay_ms,
                        at.as_ref(),
                        target.as_ref(),
                        speed,
                    )?,
                    _ => {
                        if !self.wait_interruptibly(action.delay_ms(), speed) {
                            false
                        } else {
                            unsafe { post_action(action)? };
                            true
                        }
                    }
                };
                if completed {
                    if let Action::Key { key_code, down, .. } = action {
                        if *down {
                            pressed_keys.insert(*key_code);
                        } else {
                            pressed_keys.remove(key_code);
                        }
                    }
                }
                if !completed || !self.playing.load(Ordering::SeqCst) {
                    return Ok(());
                }
                progress(PlaybackProgress {
                    index: start_at + offset,
                    total: actions.len(),
                });
            }
            Ok(())
        })();
        let release_result = unsafe { release_pressed_keys(&pressed_keys) };
        result.and(release_result)
    }

    fn play_text(&self, text: &str, target: Option<&SemanticTarget>) -> Result<bool, String> {
        if let Some(target) = target {
            let resolved = self
                .resolve_target(target, CGPoint { x: 0.0, y: 0.0 })
                .ok_or_else(|| {
                    format!(
                        "Stopped before typing because the semantic text target could not be resolved safely: {}",
                        semantic_target_name(target)
                    )
                })?;
            if !unsafe { focus_and_verify_semantic_element(&resolved) } {
                return Err(format!(
                    "Stopped before typing because macOS did not focus the intended text field: {}",
                    semantic_target_name(target)
                ));
            }
        }
        for character in text.chars() {
            if !self.playing.load(Ordering::SeqCst) {
                return Ok(false);
            }
            unsafe { post_unicode_character(character)? };
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn play_click(
        &self,
        x: f64,
        y: f64,
        button: MouseButton,
        clicks: u8,
        delay_ms: u64,
        approach: &[PointerSample],
        target: Option<&SemanticTarget>,
        speed: f64,
        pointer_hz: u16,
    ) -> Result<bool, String> {
        let original = CGPoint { x, y };
        let resolved = target.and_then(|target| self.resolve_target(target, original));
        if target.is_some() && resolved.is_none() {
            let label = target
                .and_then(|target| {
                    target
                        .title
                        .as_deref()
                        .or(target.description.as_deref())
                        .or(target.identifier.as_deref())
                })
                .unwrap_or("unlabelled element");
            return Err(format!(
                "Stopped before clicking because the semantic target could not be resolved safely: {label}"
            ));
        }
        let destination = resolved
            .as_ref()
            .and_then(|target| target.point)
            .unwrap_or(original);
        let approached = if resolved.is_some() {
            if resolved
                .as_ref()
                .is_some_and(|target| target.point.is_some())
            {
                self.play_approach(&[], destination, delay_ms, speed, pointer_hz)?
            } else {
                self.wait_interruptibly(delay_ms, speed)
            }
        } else {
            self.play_approach(approach, destination, delay_ms, speed, pointer_hz)?
        };
        if !approached || !self.wait_interruptibly(55, 1.0) {
            return Ok(false);
        }
        if let (Some(recorded), Some(resolved)) = (target, resolved.as_ref()) {
            // AXPress is only an advertised capability. Several native apps,
            // including Calculator, expose it for buttons but reject it at
            // runtime. A semantically resolved element gives us something
            // stronger than the recording's old screen coordinates: refresh
            // its current frame and replay the actual pointer action inside
            // that verified element. This also preserves the behaviour of
            // controls that distinguish a physical click from AXPress.
            if let Some(point) = unsafe { semantic_activation_point(resolved.element.as_ax(), recorded) } {
                unsafe { post_click(point, button, clicks)? };
                return Ok(true);
            }

            // Some accessibility-only controls have no screen frame. Native
            // activation is the only safe option for those, and only applies
            // to a normal primary-button click.
            if button == MouseButton::Left {
                if let Some(action) = semantic_action_for_click(recorded, &resolved.actions, clicks) {
                    if unsafe { perform_ax_action(resolved.element.as_ax(), action) } {
                        return Ok(true);
                    }
                }
            }
            return Err(format!(
                "The resolved target has no clickable screen bounds and did not accept its native action: {}",
                semantic_target_name(recorded)
            ));
        }
        unsafe { post_click(destination, button, clicks)? };
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn play_scroll(
        &self,
        x: i32,
        y: i32,
        unit: ScrollUnit,
        delay_ms: u64,
        at: Option<&ScreenPoint>,
        target: Option<&SemanticTarget>,
        speed: f64,
    ) -> Result<bool, String> {
        let original = at
            .map(|point| CGPoint {
                x: point.x,
                y: point.y,
            })
            .unwrap_or_else(|| unsafe {
                current_pointer_location().unwrap_or(CGPoint { x: 0.0, y: 0.0 })
            });
        let destination = target
            .and_then(|target| self.resolve_target(target, original))
            .and_then(|target| target.point)
            .or_else(|| {
                at.map(|point| CGPoint {
                    x: point.x,
                    y: point.y,
                })
            });
        if !self.wait_interruptibly(delay_ms, speed) {
            return Ok(false);
        }
        if let Some(destination) = destination {
            unsafe { post_pointer_event(MOUSE_MOVED, destination, 0)? };
        }
        unsafe { post_scroll(x, y, unit)? };
        Ok(true)
    }

    fn resolve_target(
        &self,
        target: &SemanticTarget,
        original: CGPoint,
    ) -> Option<ResolvedSemanticTarget> {
        if let Some(application) = target.application.as_deref() {
            activate_application(application);
        }
        for _ in 0..15 {
            if !self.playing.load(Ordering::SeqCst) {
                return None;
            }
            if let Some(resolved) = unsafe { resolve_semantic_target(target, original) } {
                return Some(resolved);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        None
    }

    fn play_approach(
        &self,
        approach: &[PointerSample],
        target: CGPoint,
        delay_ms: u64,
        speed: f64,
        pointer_hz: u16,
    ) -> Result<bool, String> {
        let mut current = unsafe { current_pointer_location()? };
        let mut elapsed = 0;
        for sample in approach {
            let target_offset = sample.offset_ms.min(delay_ms);
            let destination = CGPoint {
                x: sample.x,
                y: sample.y,
            };
            if !self.smooth_pointer(
                current,
                destination,
                target_offset.saturating_sub(elapsed),
                speed,
                pointer_hz,
                MOUSE_MOVED,
                0,
            )? {
                return Ok(false);
            }
            current = destination;
            elapsed = target_offset;
            if sample.screen_edge && !self.wait_interruptibly(450, 1.0) {
                return Ok(false);
            }
        }
        if !self.smooth_pointer(
            current,
            target,
            delay_ms.saturating_sub(elapsed),
            speed,
            pointer_hz,
            MOUSE_MOVED,
            0,
        )? {
            return Ok(false);
        }
        if point_touches_display_edge(target.x, target.y) {
            return Ok(self.wait_interruptibly(450, 1.0));
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn smooth_pointer(
        &self,
        start: CGPoint,
        end: CGPoint,
        duration_ms: u64,
        speed: f64,
        pointer_hz: u16,
        event_type: u32,
        button_code: u32,
    ) -> Result<bool, String> {
        let scaled_ms = (duration_ms as f64 / speed).round() as u64;
        let steps = pointer_step_count(duration_ms, speed, pointer_hz);
        let started = Instant::now();
        let total = Duration::from_millis(scaled_ms);
        for step in 1..=steps {
            if !self.playing.load(Ordering::SeqCst) {
                return Ok(false);
            }
            let deadline = total.mul_f64(step as f64 / steps as f64);
            while started.elapsed() < deadline {
                if !self.playing.load(Ordering::SeqCst) {
                    return Ok(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            let progress = step as f64 / steps as f64;
            let point = CGPoint {
                x: start.x + (end.x - start.x) * progress,
                y: start.y + (end.y - start.y) * progress,
            };
            unsafe { post_pointer_event(event_type, point, button_code)? };
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn play_drag(
        &self,
        button: MouseButton,
        approach: &[PointerSample],
        points: &[PointerSample],
        duration_ms: u64,
        delay_ms: u64,
        speed: f64,
        pointer_hz: u16,
    ) -> Result<bool, String> {
        let Some(first) = points.first() else {
            return Err("Drag action has no pointer path".into());
        };
        if !self.play_approach(
            approach,
            CGPoint {
                x: first.x,
                y: first.y,
            },
            delay_ms,
            speed,
            pointer_hz,
        )? {
            return Ok(false);
        }
        let (button_code, down, up, dragged) = mouse_event_types(button);
        unsafe {
            post_pointer_event(
                down,
                CGPoint {
                    x: first.x,
                    y: first.y,
                },
                button_code,
            )?;
        }
        let mut current = CGPoint {
            x: first.x,
            y: first.y,
        };
        let mut elapsed = 0;
        for sample in points.iter().skip(1) {
            let offset = sample.offset_ms.min(duration_ms);
            let destination = CGPoint {
                x: sample.x,
                y: sample.y,
            };
            let moved = self.smooth_pointer(
                current,
                destination,
                offset.saturating_sub(elapsed),
                speed,
                pointer_hz,
                dragged,
                button_code,
            );
            match moved {
                Ok(true) => {}
                Ok(false) => {
                    let release_at = unsafe { current_pointer_location().unwrap_or(current) };
                    unsafe { post_pointer_event(up, release_at, button_code)? };
                    return Ok(false);
                }
                Err(error) => {
                    let release_at = unsafe { current_pointer_location().unwrap_or(current) };
                    let _ = unsafe { post_pointer_event(up, release_at, button_code) };
                    return Err(error);
                }
            }
            current = destination;
            elapsed = offset;
        }
        if elapsed < duration_ms && !self.wait_interruptibly(duration_ms - elapsed, speed) {
            unsafe { post_pointer_event(up, current, button_code)? };
            return Ok(false);
        }
        unsafe { post_pointer_event(up, current, button_code)? };
        Ok(true)
    }

    fn play_system_action(&self, command: SystemCommand, pointer_hz: u16) -> Result<bool, String> {
        match command {
            SystemCommand::MissionControl => {
                let opened = std::process::Command::new("open")
                    .args(["-a", "Mission Control"])
                    .status()
                    .is_ok_and(|status| status.success());
                if !opened {
                    unsafe { post_key_chord(126, CONTROL)? };
                }
            }
            SystemCommand::ShowDesktop => unsafe {
                post_key_chord(103, SECONDARY_FN)?;
            },
            SystemCommand::RevealDock => return self.reveal_dock(pointer_hz),
            SystemCommand::DesktopAndDock => unsafe {
                post_key_chord(103, SECONDARY_FN)?;
                if !self.wait_interruptibly(350, 1.0) {
                    return Ok(false);
                }
                return self.reveal_dock(pointer_hz);
            },
        }
        Ok(self.wait_interruptibly(350, 1.0))
    }

    fn reveal_dock(&self, pointer_hz: u16) -> Result<bool, String> {
        let start = unsafe { current_pointer_location()? };
        let bounds = display_bounds_at(start).ok_or("Unable to locate the current display")?;
        let right = bounds.origin.x + bounds.size.width;
        let bottom = bounds.origin.y + bounds.size.height;
        let target = match dock_edge() {
            DockEdge::Bottom => CGPoint {
                x: start.x.clamp(bounds.origin.x + 2.0, right - 2.0),
                y: bottom - 1.0,
            },
            DockEdge::Left => CGPoint {
                x: bounds.origin.x + 1.0,
                y: start.y.clamp(bounds.origin.y + 2.0, bottom - 2.0),
            },
            DockEdge::Right => CGPoint {
                x: right - 1.0,
                y: start.y.clamp(bounds.origin.y + 2.0, bottom - 2.0),
            },
        };
        if !self.smooth_pointer(start, target, 180, 1.0, pointer_hz, MOUSE_MOVED, 0)? {
            return Ok(false);
        }
        Ok(self.wait_interruptibly(700, 1.0))
    }

    fn wait_interruptibly(&self, millis: u64, speed: f64) -> bool {
        let remaining = Duration::from_millis((millis as f64 / speed) as u64);
        let started = Instant::now();
        while started.elapsed() < remaining {
            if !self.playing.load(Ordering::SeqCst) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }
}

fn dock_edge() -> DockEdge {
    std::process::Command::new("defaults")
        .args(["read", "com.apple.dock", "orientation"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| dock_edge_from_preference(&value))
        .unwrap_or(DockEdge::Bottom)
}

fn dock_edge_from_preference(value: &str) -> DockEdge {
    match value.trim() {
        "left" => DockEdge::Left,
        "right" => DockEdge::Right,
        _ => DockEdge::Bottom,
    }
}

fn pointer_step_count(duration_ms: u64, speed: f64, pointer_hz: u16) -> u64 {
    let scaled_ms = (duration_ms as f64 / speed.clamp(0.25, 5.0)).round() as u64;
    scaled_ms
        .saturating_mul(u64::from(pointer_hz.clamp(30, 240)))
        .div_ceil(1000)
        .max(1)
}

unsafe fn continuous_scroll_delta(event: CGEventRef, vertical: bool, remainder: &mut f64) -> i32 {
    let point_field = if vertical {
        FIELD_SCROLL_POINT_DELTA_Y
    } else {
        FIELD_SCROLL_POINT_DELTA_X
    };
    let fixed_field = if vertical {
        FIELD_SCROLL_FIXED_DELTA_Y
    } else {
        FIELD_SCROLL_FIXED_DELTA_X
    };
    let point_delta = CGEventGetIntegerValueField(event, point_field);
    let raw_delta = if point_delta != 0 {
        point_delta as f64
    } else {
        CGEventGetDoubleValueField(event, fixed_field)
    };
    accumulate_scroll_delta(raw_delta, remainder)
}

fn accumulate_scroll_delta(raw_delta: f64, remainder: &mut f64) -> i32 {
    if !raw_delta.is_finite() {
        return 0;
    }
    let total = (*remainder + raw_delta).clamp(i32::MIN as f64, i32::MAX as f64);
    let whole = total.trunc() as i32;
    *remainder = total - f64::from(whole);
    whole
}

thread_local! {
    static LISTENER: std::cell::RefCell<Option<Arc<MacAutomation>>> = const { std::cell::RefCell::new(None) };
    static EVENT_TAP: std::cell::Cell<CFMachPortRef> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

fn point_touches_display_edge(x: f64, y: f64) -> bool {
    let Some(bounds) = display_bounds_at(CGPoint { x, y }) else {
        return false;
    };
    let right = bounds.origin.x + bounds.size.width;
    let bottom = bounds.origin.y + bounds.size.height;
    const EDGE_TOLERANCE: f64 = 3.0;
    (x - bounds.origin.x).abs() <= EDGE_TOLERANCE
        || (x - right).abs() <= EDGE_TOLERANCE
        || (y - bounds.origin.y).abs() <= EDGE_TOLERANCE
        || (y - bottom).abs() <= EDGE_TOLERANCE
}

fn display_bounds_at(point: CGPoint) -> Option<CGRect> {
    unsafe {
        let mut displays = [0u32; 8];
        let mut count = 0u32;
        if CGGetDisplaysWithPoint(
            point,
            displays.len() as u32,
            displays.as_mut_ptr(),
            &mut count,
        ) != 0
        {
            return None;
        }
        displays
            .first()
            .filter(|_| count > 0)
            .map(|display| CGDisplayBounds(*display))
    }
}

fn mouse_button_for_event(event: CGEventRef) -> MouseButton {
    match unsafe { CGEventGetIntegerValueField(event, FIELD_MOUSE_BUTTON) } {
        1 => MouseButton::Right,
        2 => MouseButton::Center,
        value if value > 2 => MouseButton::Other,
        _ => MouseButton::Left,
    }
}

fn mouse_event_types(button: MouseButton) -> (u32, u32, u32, u32) {
    match button {
        MouseButton::Left => (0, LEFT_DOWN, LEFT_UP, LEFT_DRAGGED),
        MouseButton::Right => (1, RIGHT_DOWN, RIGHT_UP, RIGHT_DRAGGED),
        MouseButton::Center | MouseButton::Other => (2, OTHER_DOWN, OTHER_UP, OTHER_DRAGGED),
    }
}

struct OwnedCf(CFTypeRef);

impl OwnedCf {
    fn as_ax(&self) -> AXUIElementRef {
        self.0.cast()
    }
}

impl Drop for OwnedCf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

struct ResolvedSemanticTarget {
    element: OwnedCf,
    point: Option<CGPoint>,
    actions: Vec<String>,
}

fn semantic_target_name(target: &SemanticTarget) -> &str {
    target
        .title
        .as_deref()
        .or(target.description.as_deref())
        .or(target.identifier.as_deref())
        .unwrap_or(target.role.as_str())
}

fn can_append_text(actions: &[Action], delay_ms: u64) -> bool {
    delay_ms < 500 && matches!(actions.last(), Some(Action::Text { .. }))
}

fn semantic_target_at(x: f64, y: f64) -> Option<SemanticTarget> {
    unsafe {
        let system = OwnedCf(AXUIElementCreateSystemWide().cast());
        if system.0.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(system.as_ax(), 0.08);
        let mut raw_element: AXUIElementRef = std::ptr::null();
        if AXUIElementCopyElementAtPosition(system.as_ax(), x as f32, y as f32, &mut raw_element)
            != AX_SUCCESS
            || raw_element.is_null()
        {
            return None;
        }
        let element = OwnedCf(raw_element.cast());
        semantic_target_from_element(element.as_ax(), Some(CGPoint { x, y }))
    }
}

fn semantic_focused_target() -> Option<SemanticTarget> {
    unsafe {
        let system = OwnedCf(AXUIElementCreateSystemWide().cast());
        if system.0.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(system.as_ax(), 0.08);
        let focused = copy_ax_value(system.as_ax(), "AXFocusedUIElement")?;
        if CFGetTypeID(focused.0) != AXUIElementGetTypeID() {
            return None;
        }
        semantic_target_from_element(focused.as_ax(), None)
    }
}

unsafe fn semantic_target_from_element(
    element: AXUIElementRef,
    activation: Option<CGPoint>,
) -> Option<SemanticTarget> {
    AXUIElementSetMessagingTimeout(element, 0.08);
    let node = semantic_node(element)?;
    let mut pid = 0;
    let application = if AXUIElementGetPid(element, &mut pid) == AX_SUCCESS {
        process_path(pid)
    } else {
        None
    };
    let actions = copy_ax_actions(element);
    let (path, ancestors) = semantic_path_and_ancestors(element);
    let window = copy_ax_value(element, "AXWindow")
        .filter(|value| CFGetTypeID(value.0) == AXUIElementGetTypeID())
        .and_then(|value| semantic_node(value.as_ax()));
    let activation_point = activation.and_then(|point| {
        let frame = ax_element_frame(element)?;
        if frame.size.width <= 0.0 || frame.size.height <= 0.0 {
            return None;
        }
        Some(RelativePoint {
            x: ((point.x - frame.origin.x) / frame.size.width).clamp(0.0, 1.0),
            y: ((point.y - frame.origin.y) / frame.size.height).clamp(0.0, 1.0),
        })
    });
    let mutable_text_value = matches!(
        node.role.as_str(),
        "AXTextField" | "AXTextArea" | "AXComboBox"
    ) || matches!(
        node.subrole.as_deref(),
        Some("AXSecureTextField" | "AXSearchField")
    );
    Some(SemanticTarget {
        application,
        role: node.role,
        subrole: node.subrole,
        identifier: node.identifier,
        title: node.title,
        description: node.description,
        help: copy_ax_string(element, "AXHelp"),
        value: (!mutable_text_value)
            .then(|| copy_ax_string(element, "AXValue"))
            .flatten(),
        filename: copy_ax_string(element, "AXFilename"),
        actions,
        path,
        activation_point,
        window,
        ancestors,
    })
}

unsafe fn copy_ax_value(element: AXUIElementRef, attribute: &str) -> Option<OwnedCf> {
    let attribute = create_cf_string(attribute)?;
    let mut value: CFTypeRef = std::ptr::null();
    if AXUIElementCopyAttributeValue(element, attribute.0.cast(), &mut value) == AX_SUCCESS
        && !value.is_null()
    {
        Some(OwnedCf(value))
    } else {
        None
    }
}

unsafe fn copy_ax_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let value = copy_ax_value(element, attribute)?;
    if CFGetTypeID(value.0) != CFStringGetTypeID() {
        return None;
    }
    cf_string_to_string(value.0.cast()).and_then(limit_semantic_text)
}

unsafe fn create_cf_string(value: &str) -> Option<OwnedCf> {
    let value = std::ffi::CString::new(value).ok()?;
    let string = CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), UTF8_ENCODING);
    (!string.is_null()).then_some(OwnedCf(string.cast()))
}

unsafe fn perform_ax_action(element: AXUIElementRef, action: &str) -> bool {
    create_cf_string(action)
        .is_some_and(|action| AXUIElementPerformAction(element, action.0.cast()) == AX_SUCCESS)
}

unsafe fn focused_element_is(element: AXUIElementRef) -> bool {
    let mut pid = 0;
    if AXUIElementGetPid(element, &mut pid) != AX_SUCCESS {
        return false;
    }
    let application = OwnedCf(AXUIElementCreateApplication(pid).cast());
    if application.0.is_null() {
        return false;
    }
    AXUIElementSetMessagingTimeout(application.as_ax(), 0.15);
    copy_ax_value(application.as_ax(), "AXFocusedUIElement")
        .filter(|focused| CFGetTypeID(focused.0) == AXUIElementGetTypeID())
        .is_some_and(|focused| CFEqual(focused.0, element.cast()))
}

unsafe fn focus_and_verify_semantic_element(target: &ResolvedSemanticTarget) -> bool {
    if focused_element_is(target.element.as_ax()) {
        return true;
    }
    let focused = create_cf_string("AXFocused").is_some_and(|attribute| {
        AXUIElementSetAttributeValue(target.element.as_ax(), attribute.0.cast(), kCFBooleanTrue)
            == AX_SUCCESS
    });
    if focused {
        std::thread::sleep(Duration::from_millis(80));
        if focused_element_is(target.element.as_ax()) {
            return true;
        }
    }
    let Some(point) = target.point else {
        return false;
    };
    if post_click(point, MouseButton::Left, 1).is_err() {
        return false;
    }
    std::thread::sleep(Duration::from_millis(120));
    focused_element_is(target.element.as_ax())
}

fn semantic_action_for_click<'a>(
    target: &SemanticTarget,
    actions: &'a [String],
    clicks: u8,
) -> Option<&'a str> {
    let supports = |name: &str| {
        actions
            .iter()
            .find(|candidate| candidate.as_str() == name)
            .map(String::as_str)
    };
    if clicks > 1 {
        return supports("AXOpen");
    }
    let directly_activatable = target.role == "AXButton"
        || target.role == "AXMenuItem"
        || target.role == "AXCheckBox"
        || target.role == "AXRadioButton"
        || target.role == "AXDockItem"
        || target.subrole.as_deref().is_some_and(|subrole| {
            matches!(
                subrole,
                "AXCloseButton" | "AXMinimizeButton" | "AXZoomButton" | "AXFullScreenButton"
            )
        });
    if !directly_activatable {
        return None;
    }
    ["AXPress", "AXPick", "AXConfirm", "AXOpen"]
        .into_iter()
        .find_map(supports)
}

unsafe fn cf_string_to_string(value: CFStringRef) -> Option<String> {
    let length = CFStringGetLength(value);
    let maximum = CFStringGetMaximumSizeForEncoding(length, UTF8_ENCODING);
    if maximum < 0 {
        return None;
    }
    let mut buffer = vec![0i8; maximum as usize + 1];
    if !CFStringGetCString(
        value,
        buffer.as_mut_ptr(),
        buffer.len() as isize,
        UTF8_ENCODING,
    ) {
        return None;
    }
    std::ffi::CStr::from_ptr(buffer.as_ptr())
        .to_str()
        .ok()
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

fn limit_semantic_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(512).collect())
}

unsafe fn copy_ax_actions(element: AXUIElementRef) -> Vec<String> {
    let mut raw: CFArrayRef = std::ptr::null();
    if AXUIElementCopyActionNames(element, &mut raw) != AX_SUCCESS || raw.is_null() {
        return Vec::new();
    }
    let actions = OwnedCf(raw.cast());
    if CFGetTypeID(actions.0) != CFArrayGetTypeID() {
        return Vec::new();
    }
    let mut result = Vec::new();
    for index in 0..CFArrayGetCount(raw) {
        let value = CFArrayGetValueAtIndex(raw, index);
        if !value.is_null() && CFGetTypeID(value) == CFStringGetTypeID() {
            if let Some(action) = cf_string_to_string(value.cast()) {
                result.push(action);
            }
        }
    }
    result.sort();
    result.dedup();
    result
}

unsafe fn semantic_node(element: AXUIElementRef) -> Option<SemanticNode> {
    let subrole = copy_ax_string(element, "AXSubrole");
    Some(SemanticNode {
        role: copy_ax_string(element, "AXRole")?,
        subrole,
        identifier: copy_ax_string(element, "AXIdentifier"),
        title: copy_ax_string(element, "AXTitle"),
        description: copy_ax_string(element, "AXDescription"),
        help: None,
        value: None,
        filename: None,
    })
}

unsafe fn semantic_candidate_node(
    element: AXUIElementRef,
    target: &SemanticTarget,
) -> Option<SemanticNode> {
    let mut node = semantic_node(element)?;
    if node.role == target.role {
        if target.help.is_some() {
            node.help = copy_ax_string(element, "AXHelp");
        }
        if target.value.is_some() && node.subrole.as_deref() != Some("AXSecureTextField") {
            node.value = copy_ax_string(element, "AXValue");
        }
        if target.filename.is_some() {
            node.filename = copy_ax_string(element, "AXFilename");
        }
    }
    Some(node)
}

unsafe fn semantic_path_and_ancestors(element: AXUIElementRef) -> (Vec<usize>, Vec<SemanticNode>) {
    let retained = CFRetain(element.cast());
    if retained.is_null() {
        return (Vec::new(), Vec::new());
    }
    let mut current = OwnedCf(retained);
    let mut reversed_path = Vec::new();
    let mut ancestors = Vec::new();
    let mut path_complete = true;
    let mut reached_application = false;
    for _ in 0..32 {
        let Some(parent) = copy_ax_value(current.as_ax(), "AXParent") else {
            path_complete = false;
            break;
        };
        if CFGetTypeID(parent.0) != AXUIElementGetTypeID() {
            path_complete = false;
            break;
        }
        if let Some(index) = child_index(parent.as_ax(), current.as_ax()) {
            reversed_path.push(index);
        } else {
            path_complete = false;
        }
        let Some(parent_node) = semantic_node(parent.as_ax()) else {
            path_complete = false;
            break;
        };
        if parent_node.role == "AXApplication" {
            reached_application = true;
            break;
        }
        ancestors.push(parent_node);
        current = parent;
    }
    if path_complete && reached_application {
        reversed_path.reverse();
    } else {
        reversed_path.clear();
    }
    (reversed_path, ancestors)
}

unsafe fn child_index(parent: AXUIElementRef, child: AXUIElementRef) -> Option<usize> {
    let children = copy_ax_value(parent, "AXChildren")?;
    if CFGetTypeID(children.0) != CFArrayGetTypeID() {
        return None;
    }
    let array: CFArrayRef = children.0.cast();
    (0..CFArrayGetCount(array)).find_map(|index| {
        let candidate = CFArrayGetValueAtIndex(array, index);
        (!candidate.is_null() && CFEqual(candidate, child.cast())).then_some(index as usize)
    })
}

fn process_path(pid: i32) -> Option<String> {
    let mut buffer = vec![0u8; 4096];
    let length = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 {
        return None;
    }
    buffer.truncate(length as usize);
    String::from_utf8(buffer).ok()
}

fn application_bundle_path(executable: &str) -> Option<&str> {
    executable
        .find(".app/")
        .map(|index| &executable[..index + ".app".len()])
}

fn activate_application(executable: &str) {
    if let Some(bundle) = application_bundle_path(executable) {
        let mut command = std::process::Command::new("open");
        command.arg("-a");
        if std::path::Path::new(bundle).exists() {
            command.arg(bundle);
        } else if let Some(name) = std::path::Path::new(bundle)
            .file_stem()
            .and_then(|name| name.to_str())
        {
            command.arg(name);
        }
        let _ = command.status();
    }
}

fn find_process_by_path(executable: &str) -> Option<i32> {
    let output = std::process::Command::new("ps")
        .args(["-axo", "pid=,comm="])
        .output()
        .ok()?;
    let processes = String::from_utf8(output.stdout).ok()?;
    processes.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let split = trimmed.find(char::is_whitespace)?;
        let pid = trimmed[..split].parse::<i32>().ok()?;
        executable_paths_match(executable, trimmed[split..].trim()).then_some(pid)
    })
}

fn executable_paths_match(recorded: &str, running: &str) -> bool {
    if recorded == running {
        return true;
    }
    let recorded_bundle = application_bundle_path(recorded);
    let running_bundle = application_bundle_path(running);
    let same_bundle_name = recorded_bundle.and_then(|path| std::path::Path::new(path).file_name())
        == running_bundle.and_then(|path| std::path::Path::new(path).file_name());
    let recorded_suffix = recorded.split_once(".app/").map(|(_, suffix)| suffix);
    let running_suffix = running.split_once(".app/").map(|(_, suffix)| suffix);
    same_bundle_name && recorded_suffix.is_some() && recorded_suffix == running_suffix
}

unsafe fn resolve_semantic_target(
    target: &SemanticTarget,
    original: CGPoint,
) -> Option<ResolvedSemanticTarget> {
    let executable = target.application.as_deref()?;
    let pid = find_process_by_path(executable)?;
    let application = OwnedCf(AXUIElementCreateApplication(pid).cast());
    if application.0.is_null() {
        return None;
    }
    AXUIElementSetMessagingTimeout(application.as_ax(), 0.2);

    if !target.path.is_empty() {
        if let Some((element, lineage)) =
            element_at_semantic_path(application.as_ax(), &target.path)
        {
            if let Some(node) = semantic_candidate_node(element.as_ax(), target) {
                let point = semantic_activation_point(element.as_ax(), target);
                let center = ax_element_center(element.as_ax());
                if semantic_match_score(&node, &lineage, target, &target.path, center, original)
                    .is_some_and(|evidence| evidence.score >= 80)
                {
                    let actions = copy_ax_actions(element.as_ax());
                    return Some(ResolvedSemanticTarget {
                        element,
                        point,
                        actions,
                    });
                }
            }
        }
    }

    let mut matches = SemanticSearch::default();
    let mut visited = 0usize;
    let mut lineage = Vec::new();
    let mut path = Vec::new();
    search_accessibility_tree(
        application.as_ax(),
        target,
        original,
        0,
        &mut visited,
        &mut lineage,
        &mut path,
        &mut matches,
    );
    let best = matches.best?;
    if best.score < 80 || (!best.exact_path && matches.second_score >= best.score - 25) {
        return None;
    }
    let point = semantic_activation_point(best.element.as_ax(), target);
    let actions = copy_ax_actions(best.element.as_ax());
    Some(ResolvedSemanticTarget {
        element: best.element,
        point,
        actions,
    })
}

unsafe fn element_at_semantic_path(
    application: AXUIElementRef,
    path: &[usize],
) -> Option<(OwnedCf, Vec<SemanticNode>)> {
    let retained = CFRetain(application.cast());
    if retained.is_null() {
        return None;
    }
    let mut current = OwnedCf(retained);
    let mut lineage = Vec::new();
    for index in path {
        if let Some(node) = semantic_node(current.as_ax()) {
            lineage.push(node);
        }
        let children = copy_ax_value(current.as_ax(), "AXChildren")?;
        if CFGetTypeID(children.0) != CFArrayGetTypeID() {
            return None;
        }
        let array: CFArrayRef = children.0.cast();
        if *index >= CFArrayGetCount(array) as usize {
            return None;
        }
        let child = CFArrayGetValueAtIndex(array, *index as isize);
        if child.is_null() || CFGetTypeID(child) != AXUIElementGetTypeID() {
            return None;
        }
        let retained_child = CFRetain(child);
        if retained_child.is_null() {
            return None;
        }
        current = OwnedCf(retained_child);
    }
    Some((current, lineage))
}

struct SemanticCandidate {
    score: i32,
    exact_path: bool,
    element: OwnedCf,
}

#[derive(Default)]
struct SemanticSearch {
    best: Option<SemanticCandidate>,
    second_score: i32,
}

impl SemanticSearch {
    fn consider(&mut self, candidate: SemanticCandidate) {
        if self
            .best
            .as_ref()
            .is_none_or(|best| candidate.score > best.score)
        {
            if let Some(previous) = self.best.replace(candidate) {
                self.second_score = self.second_score.max(previous.score);
            }
        } else {
            self.second_score = self.second_score.max(candidate.score);
        }
    }
}

struct MatchEvidence {
    score: i32,
    exact_path: bool,
}

#[allow(clippy::too_many_arguments)]
unsafe fn search_accessibility_tree(
    element: AXUIElementRef,
    target: &SemanticTarget,
    original: CGPoint,
    depth: usize,
    visited: &mut usize,
    lineage: &mut Vec<SemanticNode>,
    path: &mut Vec<usize>,
    matches: &mut SemanticSearch,
) {
    if depth > 32 || *visited >= 5000 {
        return;
    }
    *visited += 1;
    let node = semantic_candidate_node(element, target);
    if let Some(node) = node.as_ref() {
        if let Some(evidence) = semantic_match_score(
            node,
            lineage,
            target,
            path,
            ax_element_center(element),
            original,
        ) {
            let retained = CFRetain(element.cast());
            if !retained.is_null() {
                matches.consider(SemanticCandidate {
                    score: evidence.score,
                    exact_path: evidence.exact_path,
                    element: OwnedCf(retained),
                });
            }
        }
    }
    let Some(children) = copy_ax_value(element, "AXChildren") else {
        return;
    };
    if CFGetTypeID(children.0) != CFArrayGetTypeID() {
        return;
    }
    let count = CFArrayGetCount(children.0.cast()).min(1000);
    if let Some(node) = node.as_ref() {
        lineage.push(node.clone());
    }
    for index in 0..count {
        if *visited >= 5000 {
            break;
        }
        let child = CFArrayGetValueAtIndex(children.0.cast(), index);
        if !child.is_null() && CFGetTypeID(child) == AXUIElementGetTypeID() {
            path.push(index as usize);
            search_accessibility_tree(
                child.cast(),
                target,
                original,
                depth + 1,
                visited,
                lineage,
                path,
                matches,
            );
            path.pop();
        }
    }
    if node.is_some() {
        lineage.pop();
    }
}

fn semantic_match_score(
    node: &SemanticNode,
    lineage: &[SemanticNode],
    target: &SemanticTarget,
    candidate_path: &[usize],
    center: Option<CGPoint>,
    original: CGPoint,
) -> Option<MatchEvidence> {
    if node.role != target.role {
        return None;
    }
    let mut score = 10;
    let mut strong = false;
    if let Some(subrole) = target.subrole.as_deref() {
        if node.subrole.as_deref() != Some(subrole) {
            return None;
        }
        score += 240;
        strong = true;
    }
    if let Some(identifier) = target.identifier.as_deref() {
        if node.identifier.as_deref() != Some(identifier) {
            return None;
        }
        score += 220;
        strong = true;
    }
    score_semantic_text(
        &mut score,
        &mut strong,
        target.title.as_deref(),
        node.title.as_deref(),
        110,
        45,
    );
    score_semantic_text(
        &mut score,
        &mut strong,
        target.description.as_deref(),
        node.description.as_deref(),
        140,
        70,
    );
    score_semantic_text(
        &mut score,
        &mut strong,
        target.help.as_deref(),
        node.help.as_deref(),
        70,
        20,
    );
    score_semantic_text(
        &mut score,
        &mut strong,
        target.value.as_deref(),
        node.value.as_deref(),
        90,
        30,
    );
    score_semantic_text(
        &mut score,
        &mut strong,
        target.filename.as_deref(),
        node.filename.as_deref(),
        180,
        80,
    );

    let mut ancestor_matches = 0;
    for (expected, candidate) in target.ancestors.iter().zip(lineage.iter().rev()) {
        if expected.role != candidate.role {
            score -= 30;
            break;
        }
        ancestor_matches += 1;
        score += 12;
        if let Some(subrole) = expected.subrole.as_deref() {
            score += if candidate.subrole.as_deref() == Some(subrole) {
                25
            } else {
                -20
            };
        }
        if let Some(identifier) = expected.identifier.as_deref() {
            score += if candidate.identifier.as_deref() == Some(identifier) {
                45
            } else {
                -25
            };
        }
        if let Some(title) = expected.title.as_deref() {
            score += if semantic_text_equal(candidate.title.as_deref(), Some(title)) {
                30
            } else {
                -15
            };
        }
        if let Some(description) = expected.description.as_deref() {
            score += if semantic_text_equal(candidate.description.as_deref(), Some(description)) {
                20
            } else {
                -10
            };
        }
    }

    if let Some(expected_window) = target.window.as_ref() {
        if let Some(candidate_window) = lineage
            .iter()
            .rev()
            .find(|candidate| candidate.role == expected_window.role)
        {
            score += 20;
            if let Some(title) = expected_window.title.as_deref() {
                score += if semantic_text_equal(candidate_window.title.as_deref(), Some(title)) {
                    50
                } else {
                    -25
                };
            }
        }
    }

    let exact_path = !target.path.is_empty() && target.path == candidate_path;
    if exact_path {
        score += 120;
    }
    if let Some(center) = center {
        let distance = (center.x - original.x).hypot(center.y - original.y);
        score += (10.0 - distance / 200.0).clamp(0.0, 10.0) as i32;
    }
    if !strong && !(exact_path && ancestor_matches > 0) {
        return None;
    }
    Some(MatchEvidence { score, exact_path })
}

fn score_semantic_text(
    score: &mut i32,
    strong: &mut bool,
    expected: Option<&str>,
    candidate: Option<&str>,
    match_score: i32,
    mismatch_penalty: i32,
) {
    if expected.is_none() {
        return;
    }
    if semantic_text_equal(expected, candidate) {
        *score += match_score;
        *strong = true;
    } else {
        *score -= mismatch_penalty;
    }
}

fn semantic_text_equal(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left.trim() == right.trim())
}

unsafe fn semantic_activation_point(
    element: AXUIElementRef,
    target: &SemanticTarget,
) -> Option<CGPoint> {
    let frame = ax_element_frame(element)?;
    activation_point_in_frame(frame, target.activation_point.as_ref())
}

fn activation_point_in_frame(frame: CGRect, relative: Option<&RelativePoint>) -> Option<CGPoint> {
    if !frame.origin.x.is_finite()
        || !frame.origin.y.is_finite()
        || !frame.size.width.is_finite()
        || !frame.size.height.is_finite()
        || frame.size.width <= 0.0
        || frame.size.height <= 0.0
    {
        return None;
    }
    Some(CGPoint {
        x: frame.origin.x
            + frame.size.width * relative.map(|point| point.x).unwrap_or(0.5).clamp(0.0, 1.0),
        y: frame.origin.y
            + frame.size.height * relative.map(|point| point.y).unwrap_or(0.5).clamp(0.0, 1.0),
    })
}

unsafe fn ax_element_center(element: AXUIElementRef) -> Option<CGPoint> {
    let frame = ax_element_frame(element)?;
    Some(CGPoint {
        x: frame.origin.x + frame.size.width / 2.0,
        y: frame.origin.y + frame.size.height / 2.0,
    })
}

unsafe fn ax_element_frame(element: AXUIElementRef) -> Option<CGRect> {
    let position = copy_ax_value(element, "AXPosition")?;
    let size = copy_ax_value(element, "AXSize")?;
    if CFGetTypeID(position.0) != AXValueGetTypeID()
        || CFGetTypeID(size.0) != AXValueGetTypeID()
        || AXValueGetType(position.0.cast()) != AX_VALUE_CGPOINT
        || AXValueGetType(size.0.cast()) != AX_VALUE_CGSIZE
    {
        return None;
    }
    let mut origin = CGPoint { x: 0.0, y: 0.0 };
    let mut dimensions = CGSize {
        width: 0.0,
        height: 0.0,
    };
    if !AXValueGetValue(
        position.0.cast(),
        AX_VALUE_CGPOINT,
        (&mut origin as *mut CGPoint).cast(),
    ) || !AXValueGetValue(
        size.0.cast(),
        AX_VALUE_CGSIZE,
        (&mut dimensions as *mut CGSize).cast(),
    ) {
        return None;
    }
    Some(CGRect {
        origin,
        size: dimensions,
    })
}

fn merge_multi_click(
    actions: &mut [Action],
    x: f64,
    y: f64,
    button: MouseButton,
    click_state: u8,
) -> bool {
    match actions.last_mut() {
        Some(Action::Click {
            x: previous_x,
            y: previous_y,
            button: previous_button,
            clicks: previous_clicks,
            ..
        }) if *previous_button == button && (*previous_x - x).hypot(*previous_y - y) <= 5.0 => {
            *previous_clicks = click_state.clamp(1, 3);
            true
        }
        _ => false,
    }
}

extern "C" fn event_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    _user: *mut c_void,
) -> CGEventRef {
    if event_type == EVENT_TAP_DISABLED_BY_TIMEOUT || event_type == EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        EVENT_TAP.with(|slot| {
            let tap = slot.get();
            if !tap.is_null() {
                unsafe { CGEventTapEnable(tap, true) };
            }
        });
        return event;
    }
    LISTENER.with(|slot| {
        if let Some(listener) = slot.borrow().as_ref() {
            listener.record_event(event_type, event);
        }
    });
    event
}

unsafe fn post_action(action: &Action) -> Result<(), String> {
    match action {
        Action::Click {
            x,
            y,
            button,
            clicks,
            ..
        } => post_click(CGPoint { x: *x, y: *y }, *button, *clicks)?,
        Action::Move { x, y, .. } => {
            post_pointer_move(*x, *y)?;
        }
        Action::Drag { .. } | Action::System { .. } => {}
        Action::Scroll { x, y, unit, .. } => post_scroll(*x, *y, *unit)?,
        Action::Key {
            key_code,
            modifiers,
            down,
            ..
        } => {
            let event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), *key_code, *down);
            if event.is_null() {
                return Err("Unable to create keyboard event".into());
            }
            CGEventSetFlags(event, *modifiers);
            CGEventPost(HID_TAP, event);
            CFRelease(event);
        }
        Action::Text { text, .. } => {
            for character in text.chars() {
                post_unicode_character(character)?;
            }
        }
        Action::Wait { .. } => {}
    };
    Ok(())
}

unsafe fn post_click(point: CGPoint, button: MouseButton, clicks: u8) -> Result<(), String> {
    let (button_code, down, up, _) = mouse_event_types(button);
    let click_count = clicks.clamp(1, 3);
    for click_index in 0..click_count {
        // AppKit requires the click-state sequence as well as repeated
        // down/up pairs to recognize double- and triple-clicks.
        let click_state = i64::from(click_index + 1);
        let press = CGEventCreateMouseEvent(std::ptr::null_mut(), down, point, button_code);
        let release = CGEventCreateMouseEvent(std::ptr::null_mut(), up, point, button_code);
        if press.is_null() || release.is_null() {
            if !press.is_null() {
                CFRelease(press);
            }
            if !release.is_null() {
                CFRelease(release);
            }
            return Err("Unable to create mouse event".into());
        }
        CGEventSetIntegerValueField(press, FIELD_MOUSE_CLICK_STATE, click_state);
        CGEventSetIntegerValueField(release, FIELD_MOUSE_CLICK_STATE, click_state);
        CGEventPost(HID_TAP, press);
        CFRelease(press);
        std::thread::sleep(Duration::from_millis(12));
        CGEventPost(HID_TAP, release);
        CFRelease(release);
        if click_index + 1 < click_count {
            std::thread::sleep(Duration::from_millis(70));
        }
    }
    Ok(())
}

unsafe fn post_scroll(x: i32, y: i32, unit: ScrollUnit) -> Result<(), String> {
    let event = CGEventCreateScrollWheelEvent2(
        std::ptr::null_mut(),
        match unit {
            ScrollUnit::Pixel => 0,
            ScrollUnit::Line => 1,
        },
        2,
        y,
        x,
        0,
    );
    if event.is_null() {
        return Err("Unable to create scroll event".into());
    }
    CGEventPost(HID_TAP, event);
    CFRelease(event);
    Ok(())
}

unsafe fn post_pointer_move(x: f64, y: f64) -> Result<(), String> {
    post_pointer_event(MOUSE_MOVED, CGPoint { x, y }, 0)
}

unsafe fn post_unicode_character(character: char) -> Result<(), String> {
    let mut buffer = [0u16; 2];
    let encoded = character.encode_utf16(&mut buffer);
    let down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, true);
    let up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, false);
    if down.is_null() || up.is_null() {
        if !down.is_null() {
            CFRelease(down);
        }
        if !up.is_null() {
            CFRelease(up);
        }
        return Err("Unable to create text input event".into());
    }
    CGEventKeyboardSetUnicodeString(down, encoded.len(), encoded.as_ptr());
    CGEventKeyboardSetUnicodeString(up, encoded.len(), encoded.as_ptr());
    CGEventPost(HID_TAP, down);
    CGEventPost(HID_TAP, up);
    CFRelease(down);
    CFRelease(up);
    Ok(())
}

unsafe fn release_pressed_keys(keys: &HashSet<u16>) -> Result<(), String> {
    for key_code in keys {
        let event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), *key_code, false);
        if event.is_null() {
            return Err("Unable to release a keyboard key after playback".into());
        }
        CGEventSetFlags(event, 0);
        CGEventPost(HID_TAP, event);
        CFRelease(event);
    }
    Ok(())
}

unsafe fn post_pointer_event(
    event_type: u32,
    point: CGPoint,
    button_code: u32,
) -> Result<(), String> {
    let event = CGEventCreateMouseEvent(std::ptr::null_mut(), event_type, point, button_code);
    if event.is_null() {
        return Err("Unable to create mouse event".into());
    }
    CGEventPost(HID_TAP, event);
    CFRelease(event);
    Ok(())
}

unsafe fn current_pointer_location() -> Result<CGPoint, String> {
    let event = CGEventCreate(std::ptr::null_mut());
    if event.is_null() {
        return Err("Unable to read the current pointer location".into());
    }
    let point = CGEventGetLocation(event);
    CFRelease(event);
    Ok(point)
}

unsafe fn post_key_chord(keycode: u16, modifiers: u64) -> Result<(), String> {
    let modifier_keys = [
        (SHIFT, KEY_SHIFT),
        (CONTROL, KEY_CONTROL),
        (OPTION, KEY_OPTION),
        (COMMAND, KEY_COMMAND),
        (SECONDARY_FN, KEY_FUNCTION),
    ];
    let mut active_flags = 0;
    let mut pressed_modifiers = Vec::new();
    for (flag, modifier_key) in modifier_keys {
        if modifiers & flag != 0 {
            active_flags |= flag;
            post_key_event(modifier_key, true, active_flags)?;
            pressed_modifiers.push((flag, modifier_key));
        }
    }
    post_key_event(keycode, true, modifiers)?;
    std::thread::sleep(Duration::from_millis(24));
    post_key_event(keycode, false, modifiers)?;
    for (flag, modifier_key) in pressed_modifiers.into_iter().rev() {
        active_flags &= !flag;
        post_key_event(modifier_key, false, active_flags)?;
    }
    Ok(())
}

unsafe fn post_key_event(keycode: u16, down: bool, modifiers: u64) -> Result<(), String> {
    let event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, down);
    if event.is_null() {
        return Err("Unable to create system shortcut".into());
    }
    CGEventSetFlags(event, modifiers);
    CGEventPost(HID_TAP, event);
    CFRelease(event);
    Ok(())
}

unsafe fn unicode_text_for_event(event: CGEventRef) -> Option<String> {
    let mut buffer = [0u16; 16];
    let mut length = 0usize;
    CGEventKeyboardGetUnicodeString(event, buffer.len(), &mut length, buffer.as_mut_ptr());
    if length == 0 {
        None
    } else {
        String::from_utf16(&buffer[..length.min(buffer.len())]).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_physical_click_updates_the_existing_click_action() {
        let mut actions = vec![Action::Click {
            enabled: true,
            x: 120.0,
            y: 240.0,
            button: MouseButton::Left,
            clicks: 1,
            delay_ms: 300,
            approach: vec![],
            target: None,
        }];

        assert!(merge_multi_click(
            &mut actions,
            121.0,
            239.0,
            MouseButton::Left,
            2
        ));
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::Click { clicks, .. } => assert_eq!(*clicks, 2),
            _ => panic!("expected a click"),
        }
    }

    #[test]
    fn unrelated_clicks_are_not_merged() {
        let mut actions = vec![Action::Click {
            enabled: true,
            x: 120.0,
            y: 240.0,
            button: MouseButton::Left,
            clicks: 1,
            delay_ms: 300,
            approach: vec![],
            target: None,
        }];

        assert!(!merge_multi_click(
            &mut actions,
            180.0,
            240.0,
            MouseButton::Left,
            2
        ));
        assert!(!merge_multi_click(
            &mut actions,
            120.0,
            240.0,
            MouseButton::Right,
            2
        ));
    }

    #[test]
    fn text_grouping_requires_an_uninterrupted_typing_run() {
        let text = Action::Text {
            enabled: true,
            text: "first".into(),
            delay_ms: 0,
            target: None,
        };
        assert!(can_append_text(std::slice::from_ref(&text), 100));
        assert!(!can_append_text(std::slice::from_ref(&text), 500));

        let click = Action::Click {
            enabled: true,
            x: 10.0,
            y: 10.0,
            button: MouseButton::Left,
            clicks: 1,
            delay_ms: 20,
            approach: vec![],
            target: None,
        };
        assert!(!can_append_text(&[text, click], 100));
    }

    #[test]
    fn pointer_rate_controls_interpolation_density() {
        assert_eq!(pointer_step_count(1_000, 1.0, 120), 120);
        assert_eq!(pointer_step_count(1_000, 2.0, 120), 60);
        assert_eq!(pointer_step_count(0, 1.0, 240), 1);
        assert_eq!(pointer_step_count(1_000, 1.0, 999), 240);
    }

    #[test]
    fn playback_stays_exclusive_until_the_worker_completes() {
        let automation = MacAutomation::new();
        assert!(automation.begin_playback().is_ok());
        assert!(automation.begin_playback().is_err());

        automation.stop_playback();
        assert!(automation.begin_playback().is_err());

        automation.complete_playback();
        assert!(automation.begin_playback().is_ok());
        automation.complete_playback();
    }

    #[test]
    fn dock_orientation_preference_selects_the_reveal_edge() {
        assert_eq!(dock_edge_from_preference("left\n"), DockEdge::Left);
        assert_eq!(dock_edge_from_preference("right"), DockEdge::Right);
        assert_eq!(dock_edge_from_preference("bottom"), DockEdge::Bottom);
        assert_eq!(dock_edge_from_preference("unexpected"), DockEdge::Bottom);
    }

    #[test]
    fn fractional_trackpad_scroll_is_accumulated_instead_of_dropped() {
        let mut remainder = 0.0;
        assert_eq!(accumulate_scroll_delta(0.4, &mut remainder), 0);
        assert_eq!(accumulate_scroll_delta(0.7, &mut remainder), 1);
        assert!((remainder - 0.1).abs() < 0.000_001);
        assert_eq!(accumulate_scroll_delta(-1.6, &mut remainder), -1);
        assert!((remainder + 0.5).abs() < 0.000_001);
    }

    #[test]
    fn semantic_matching_prefers_identity_over_old_coordinates() {
        let target = SemanticTarget {
            application: Some("/Applications/Example.app/Contents/MacOS/Example".into()),
            role: "AXButton".into(),
            identifier: Some("submit".into()),
            title: Some("Submit".into()),
            ancestors: vec![SemanticNode {
                role: "AXGroup".into(),
                identifier: Some("checkout".into()),
                ..SemanticNode::default()
            }],
            ..SemanticTarget::default()
        };
        let matching = SemanticNode {
            role: "AXButton".into(),
            identifier: Some("submit".into()),
            title: Some("Submit".into()),
            ..SemanticNode::default()
        };
        let nearby_wrong = SemanticNode {
            role: "AXButton".into(),
            identifier: Some("cancel".into()),
            title: Some("Cancel".into()),
            ..SemanticNode::default()
        };
        let parent = SemanticNode {
            role: "AXGroup".into(),
            identifier: Some("checkout".into()),
            ..SemanticNode::default()
        };
        let original = CGPoint { x: 20.0, y: 20.0 };
        let matching_score = semantic_match_score(
            &matching,
            std::slice::from_ref(&parent),
            &target,
            &[],
            Some(CGPoint { x: 900.0, y: 700.0 }),
            original,
        )
        .expect("stable identifier should match")
        .score;
        let wrong_score = semantic_match_score(
            &nearby_wrong,
            std::slice::from_ref(&parent),
            &target,
            &[],
            Some(CGPoint { x: 20.0, y: 20.0 }),
            original,
        );
        assert!(wrong_score.is_none());
        assert!(matching_score >= 200);
    }

    #[test]
    fn window_control_subroles_can_never_cross_match() {
        let target = SemanticTarget {
            role: "AXButton".into(),
            subrole: Some("AXMinimizeButton".into()),
            description: Some("minimize button".into()),
            ..SemanticTarget::default()
        };
        let close = SemanticNode {
            role: "AXButton".into(),
            subrole: Some("AXCloseButton".into()),
            description: Some("close button".into()),
            ..SemanticNode::default()
        };
        assert!(semantic_match_score(
            &close,
            &[],
            &target,
            &[],
            Some(CGPoint { x: 10.0, y: 10.0 }),
            CGPoint { x: 10.0, y: 10.0 }
        )
        .is_none());
    }

    #[test]
    fn legacy_window_controls_use_description_instead_of_geometry() {
        let target = SemanticTarget {
            role: "AXButton".into(),
            description: Some("minimize button".into()),
            ..SemanticTarget::default()
        };
        let minimize = SemanticNode {
            role: "AXButton".into(),
            description: Some("minimize button".into()),
            ..SemanticNode::default()
        };
        let close = SemanticNode {
            role: "AXButton".into(),
            description: Some("close button".into()),
            ..SemanticNode::default()
        };
        let original = CGPoint { x: 10.0, y: 10.0 };
        assert!(semantic_match_score(&minimize, &[], &target, &[], None, original).is_some());
        assert!(
            semantic_match_score(&close, &[], &target, &[], Some(original), original).is_none()
        );
    }

    #[test]
    fn generic_role_and_geometry_are_not_enough_to_click() {
        let target = SemanticTarget {
            role: "AXButton".into(),
            ..SemanticTarget::default()
        };
        let candidate = SemanticNode {
            role: "AXButton".into(),
            ..SemanticNode::default()
        };
        let point = CGPoint { x: 10.0, y: 10.0 };
        assert!(semantic_match_score(&candidate, &[], &target, &[], Some(point), point).is_none());
    }

    #[test]
    fn semantic_click_uses_the_recorded_relative_position_in_current_bounds() {
        let frame = CGRect {
            origin: CGPoint { x: 400.0, y: 200.0 },
            size: CGSize {
                width: 80.0,
                height: 40.0,
            },
        };
        let relative = RelativePoint { x: 0.25, y: 0.75 };
        let point = activation_point_in_frame(frame, Some(&relative)).expect("valid frame");
        assert_eq!(point.x, 420.0);
        assert_eq!(point.y, 230.0);

        let hidden = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: 0.0,
                height: 20.0,
            },
        };
        assert!(activation_point_in_frame(hidden, Some(&relative)).is_none());
    }

    #[test]
    fn semantic_actions_preserve_single_and_double_click_intent() {
        let button = SemanticTarget {
            role: "AXButton".into(),
            ..SemanticTarget::default()
        };
        let text = SemanticTarget {
            role: "AXStaticText".into(),
            ..SemanticTarget::default()
        };
        let actions = vec!["AXOpen".into(), "AXPress".into()];
        assert_eq!(
            semantic_action_for_click(&button, &actions, 1),
            Some("AXPress")
        );
        assert_eq!(semantic_action_for_click(&text, &actions, 1), None);
        assert_eq!(
            semantic_action_for_click(&text, &actions, 2),
            Some("AXOpen")
        );
    }

    #[test]
    fn application_bundle_is_derived_from_nested_helper_paths() {
        assert_eq!(
            application_bundle_path(
                "/Applications/Browser.app/Contents/Frameworks/Browser Helper.app/Contents/MacOS/Helper"
            ),
            Some("/Applications/Browser.app")
        );
        assert_eq!(application_bundle_path("/usr/bin/example"), None);
        assert!(executable_paths_match(
            "/Applications/Browser.app/Contents/MacOS/Browser",
            "/Users/example/Applications/Browser.app/Contents/MacOS/Browser"
        ));
        assert!(!executable_paths_match(
            "/Applications/Browser.app/Contents/MacOS/Browser",
            "/Applications/Other.app/Contents/MacOS/Browser"
        ));
    }
}
