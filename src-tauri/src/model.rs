use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDocument {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    Click {
        #[serde(default = "yes")]
        enabled: bool,
        x: f64,
        y: f64,
        button: MouseButton,
        clicks: u8,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
        #[serde(default)]
        approach: Vec<PointerSample>,
        #[serde(default)]
        target: Option<SemanticTarget>,
    },
    Move {
        #[serde(default = "yes")]
        enabled: bool,
        x: f64,
        y: f64,
        #[serde(alias = "duration_ms")]
        duration_ms: u64,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
        #[serde(default)]
        screen_edge: bool,
    },
    Drag {
        #[serde(default = "yes")]
        enabled: bool,
        button: MouseButton,
        #[serde(default)]
        approach: Vec<PointerSample>,
        #[serde(default)]
        points: Vec<PointerSample>,
        #[serde(alias = "duration_ms")]
        duration_ms: u64,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
    },
    Scroll {
        #[serde(default = "yes")]
        enabled: bool,
        x: i32,
        y: i32,
        #[serde(default)]
        unit: ScrollUnit,
        #[serde(default)]
        at: Option<ScreenPoint>,
        #[serde(default)]
        target: Option<SemanticTarget>,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
    },
    Text {
        #[serde(default = "yes")]
        enabled: bool,
        text: String,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
        /// The accessibility element that owned keyboard focus while this
        /// text was recorded. Older macros omit it and retain legacy
        /// focus-based playback.
        #[serde(default)]
        target: Option<SemanticTarget>,
    },
    Key {
        #[serde(default = "yes")]
        enabled: bool,
        #[serde(alias = "key_code")]
        key_code: u16,
        modifiers: u64,
        down: bool,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
    },
    Wait {
        #[serde(default = "yes")]
        enabled: bool,
        #[serde(alias = "duration_ms")]
        duration_ms: u64,
    },
    System {
        #[serde(default = "yes")]
        enabled: bool,
        command: SystemCommand,
        #[serde(alias = "delay_ms")]
        delay_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerSample {
    pub x: f64,
    pub y: f64,
    /// Milliseconds since the start of this path. For click approaches the
    /// path starts at the previous visible action; for drags it starts at
    /// mouse-down.
    pub offset_ms: u64,
    /// True when this sample touched a display boundary and may have invoked
    /// auto-hidden macOS UI such as the Dock or menu bar.
    #[serde(default)]
    pub screen_edge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativePoint {
    /// Horizontal position inside the target's current bounds, from 0 to 1.
    pub x: f64,
    /// Vertical position inside the target's current bounds, from 0 to 1.
    pub y: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTarget {
    /// Full path of the owning application's executable. This is used only to
    /// locate the same application process during playback.
    #[serde(default)]
    pub application: Option<String>,
    pub role: String,
    /// Roles such as AXButton are intentionally broad. Subroles distinguish
    /// safety-critical controls such as close, minimize, and zoom buttons.
    #[serde(default)]
    pub subrole: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    /// Native Accessibility actions advertised by the recorded element.
    #[serde(default)]
    pub actions: Vec<String>,
    /// Child indexes from the application element to this element. This is a
    /// fast structural locator, but is always validated against semantics.
    #[serde(default)]
    pub path: Vec<usize>,
    /// Where inside the element the user clicked. Used only when the element
    /// does not offer an appropriate native Accessibility action.
    #[serde(default)]
    pub activation_point: Option<RelativePoint>,
    #[serde(default)]
    pub window: Option<SemanticNode>,
    #[serde(default)]
    pub ancestors: Vec<SemanticNode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticNode {
    pub role: String,
    #[serde(default)]
    pub subrole: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
}

impl Action {
    pub fn delay_ms(&self) -> u64 {
        match self {
            Self::Click { delay_ms, .. }
            | Self::Move { delay_ms, .. }
            | Self::Drag { delay_ms, .. }
            | Self::Scroll { delay_ms, .. }
            | Self::Text { delay_ms, .. }
            | Self::Key { delay_ms, .. }
            | Self::System { delay_ms, .. } => *delay_ms,
            Self::Wait { duration_ms, .. } => *duration_ms,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Click { enabled, .. }
            | Self::Move { enabled, .. }
            | Self::Drag { enabled, .. }
            | Self::Scroll { enabled, .. }
            | Self::Text { enabled, .. }
            | Self::Key { enabled, .. }
            | Self::Wait { enabled, .. }
            | Self::System { enabled, .. } => *enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    Left,
    Right,
    Center,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ScrollUnit {
    Pixel,
    #[default]
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemCommand {
    MissionControl,
    ShowDesktop,
    RevealDock,
    DesktopAndDock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSettings {
    #[serde(default = "yes")]
    pub mouse_clicks: bool,
    #[serde(default)]
    pub mouse_movement: bool,
    #[serde(default = "yes")]
    pub keyboard: bool,
    #[serde(default = "yes")]
    pub timing: bool,
    #[serde(default = "yes")]
    pub scrolling: bool,
}
fn yes() -> bool {
    true
}
impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            mouse_clicks: true,
            mouse_movement: false,
            keyboard: true,
            timing: true,
            scrolling: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackProgress {
    pub index: usize,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_round_trips_through_json() {
        let action = Action::Text {
            enabled: true,
            text: "hello".into(),
            delay_ms: 240,
            target: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"delayMs\":240"));
        let decoded: Action = serde_json::from_str(&json).unwrap();
        match decoded {
            Action::Text { text, delay_ms, .. } => {
                assert_eq!(text, "hello");
                assert_eq!(delay_ms, 240);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn existing_text_actions_load_without_a_semantic_target() {
        let decoded: Action =
            serde_json::from_str(r#"{"kind":"text","enabled":true,"text":"hello","delayMs":20}"#)
                .unwrap();
        match decoded {
            Action::Text { target, .. } => assert!(target.is_none()),
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn waits_participate_in_timing() {
        assert_eq!(
            Action::Wait {
                enabled: true,
                duration_ms: 1100
            }
            .delay_ms(),
            1100
        );
        assert_eq!(
            Action::Key {
                enabled: true,
                key_code: 36,
                modifiers: 0,
                down: true,
                delay_ms: 70
            }
            .delay_ms(),
            70
        );
    }

    #[test]
    fn click_approach_is_optional_for_existing_macro_files() {
        let json =
            r#"{"kind":"click","x":100.0,"y":200.0,"button":"left","clicks":1,"delayMs":300}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Click { approach, .. } => assert!(approach.is_empty()),
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn old_pointer_samples_default_to_non_edge_samples() {
        let json = r#"{"kind":"click","x":100.0,"y":200.0,"button":"left","clicks":1,"delayMs":300,"approach":[{"x":100.0,"y":799.0,"offsetMs":120}]}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Click { approach, .. } => {
                assert_eq!(approach.len(), 1);
                assert!(!approach[0].screen_edge);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn old_snake_case_action_fields_still_load() {
        let json = r#"{"kind":"key","key_code":36,"modifiers":0,"down":true,"delay_ms":70}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action.delay_ms(), 70);
    }

    #[test]
    fn existing_scroll_actions_default_to_line_units() {
        let json = r#"{"kind":"scroll","x":0,"y":-3,"delayMs":20}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Scroll { unit, y, .. } => {
                assert_eq!(unit, ScrollUnit::Line);
                assert_eq!(y, -3);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn drag_paths_and_system_actions_round_trip() {
        let actions = vec![
            Action::Drag {
                enabled: true,
                button: MouseButton::Left,
                approach: vec![],
                points: vec![
                    PointerSample {
                        x: 10.0,
                        y: 20.0,
                        offset_ms: 0,
                        screen_edge: false,
                    },
                    PointerSample {
                        x: 80.0,
                        y: 120.0,
                        offset_ms: 250,
                        screen_edge: false,
                    },
                ],
                duration_ms: 250,
                delay_ms: 100,
            },
            Action::System {
                enabled: true,
                command: SystemCommand::MissionControl,
                delay_ms: 300,
            },
        ];
        let json = serde_json::to_string(&actions).unwrap();
        let decoded: Vec<Action> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded[0].delay_ms(), 100);
        assert_eq!(decoded[1].delay_ms(), 300);
        assert!(json.contains("missionControl"));
    }

    #[test]
    fn semantic_targets_are_optional_and_round_trip() {
        let action = Action::Click {
            enabled: true,
            x: 100.0,
            y: 200.0,
            button: MouseButton::Left,
            clicks: 1,
            delay_ms: 20,
            approach: vec![],
            target: Some(SemanticTarget {
                application: Some("/Applications/Example.app/Contents/MacOS/Example".into()),
                role: "AXButton".into(),
                identifier: Some("submit".into()),
                title: Some("Submit".into()),
                ancestors: vec![SemanticNode {
                    role: "AXWindow".into(),
                    title: Some("Checkout".into()),
                    ..SemanticNode::default()
                }],
                ..SemanticTarget::default()
            }),
        };
        let json = serde_json::to_string(&action).unwrap();
        let decoded: Action = serde_json::from_str(&json).unwrap();
        match decoded {
            Action::Click {
                target: Some(target),
                ..
            } => {
                assert_eq!(target.role, "AXButton");
                assert_eq!(target.identifier.as_deref(), Some("submit"));
            }
            _ => panic!("semantic target was not preserved"),
        }
    }
}
