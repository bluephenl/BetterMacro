# Architecture notes

The macOS engine intentionally stays outside React. `MacAutomation` owns the system event tap, filters movement on the native thread, groups safe normal text, accumulates fractional trackpad scroll deltas, and emits action arrays only at UI-safe boundaries. Playback uses CoreGraphics events and checks an atomic stop flag every two milliseconds while waiting. Compact pointer approach paths are embedded in click and drag actions even when visible movement recording is off; this preserves Dock reveals, hover menus, and other pointer-history-dependent UI without cluttering the timeline. Drag paths are recorded from Quartz dragged events and interpolated at a configurable 60–240 events per second during playback.

Clicks and scroll gestures also carry an optional semantic Accessibility target: owning application executable, role and subrole, identifier, title, description, help/value/filename metadata, advertised native actions, window and ancestor signatures, a validated path through the Accessibility tree, and the original point relative to the element's bounds. Playback activates the application, validates the structural path first, then performs a bounded tree search that requires stable semantic evidence and rejects ambiguous candidates. Native actions are used for controls and document opening when their semantics match the recorded click count; other elements receive a physical click at the relocated relative point. A semantic click that cannot be resolved stops playback before input is posted. Coordinates remain supported for old actions that never contained a semantic target; scroll can still use its recorded point because a misplaced scroll is non-destructive.

`Action` is a tagged, versioned serializable union. The next schema version should introduce loops and image assets with an explicit migration function—never mutate old macro JSON in place.

## Deliberate MVP boundaries

- Image capture/matching, loops, and conditions are not yet implemented. Applications that expose no usable Accessibility identity can only be recorded as explicit screen-coordinate actions; semantic actions never silently degrade into a potentially destructive coordinate click.
- Text recording and playback use CoreGraphics Unicode event APIs. Shortcuts and control keys remain raw key events so their modifier semantics are preserved.
- Input Monitoring is checked with the public Quartz preflight API and macOS makes the final decision when the event tap starts. Accessibility is required for semantic capture and synthetic playback; Screen Recording is not needed by the current feature set.
- Display metadata and multi-monitor target validation are next-priority engine work; CoreGraphics uses global display coordinates today.
- AppKit exposes swipe events to an app's responder chain, but supported global monitors do not provide arbitrary system three-finger gestures and CoreGraphics cannot synthesize true multitouch. System actions therefore use the supported keyboard/Quartz equivalents; private `MultitouchSupport` APIs are intentionally excluded.

## Release compatibility check

`tauri.conf.json` and the Rust linker flag declare 12.6. CI should inspect `otool -l BetterMacro.app/Contents/MacOS/BetterMacro` for `minos 12.6` and build both `aarch64-apple-darwin` and `x86_64-apple-darwin` before merging them into a Universal Binary.
