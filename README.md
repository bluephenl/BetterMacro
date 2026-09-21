# BetterMacro

> Record. Understand. Edit. Replay.

BetterMacro is a local-first, open-source macro recorder for macOS. It is built as a native macOS automation engine with a compact Tauri editor—not a browser automation tool.

## Current MVP

- Global keyboard, click, high-resolution drag, mouse movement, and fractional trackpad/mouse scroll observation through a macOS `CGEventTap`.
- Readable action timeline with conservative normal-text grouping.
- Accessibility-backed semantic targets for clicks and scroll regions. Targets include role/subrole, labels, values, filename, window/ancestor context, native actions, and a validated tree path. Ambiguous semantic clicks stop safely instead of guessing with old coordinates.
- Interpolated CoreGraphics pointer playback at 60, 120, or 240 events per second, adjustable speed, immediate Stop, and global emergency stop (`⌃⌥Esc`) when that shortcut is available.
- Mission Control, Show Desktop, Reveal Dock, and combined Desktop + Dock actions using supported macOS shortcuts and Quartz movement.
- Editable and disableable actions, drag reorder, undo/redo, import/export, local save/load, library filters, and a permission UI.
- macOS deployment floor: **12.6 (Monterey)**. Configuration is Universal-Binary ready (`aarch64-apple-darwin` + `x86_64-apple-darwin`).

## Run locally

Prerequisites: Node 20+, Rust stable, Xcode or Xcode Command Line Tools, and macOS.

```sh
npm install
npm run tauri dev
```

Recording requires **Input Monitoring** and **Accessibility** so BetterMacro can observe input and identify the UI element under it. Playback requires Accessibility input control. BetterMacro requests each permission only when its feature needs it. Screen Recording is optional and is only relevant to future image targeting.

macOS does not provide a supported API for synthesizing a literal three-finger trackpad gesture. BetterMacro represents its result as deterministic Mission Control / Desktop / Dock actions instead of relying on private multitouch frameworks.

## Build and package

```sh
npm run tauri build
```

The `.app` and DMG are created under `src-tauri/target/release/bundle/`. For a release, build both targets on appropriate runners and use Tauri's universal-binary flow or `lipo`:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin
```

## Privacy

Macro files live in `~/Library/Application Support/BetterMacro/macros`. There is no account, telemetry, cloud synchronization, or automatic AI request. BetterMacro intentionally does not try to bypass macOS Secure Input protections.

## Architecture

- `src/` — React editor, timeline, inspector, permission flow and command palette.
- `src-tauri/src/macos.rs` — macOS event tap, permission checks, CoreGraphics event synthesis, interruptible playback.
- `src-tauri/src/model.rs` — versioned action and macro schema.
- `src-tauri/src/storage.rs` — atomic local JSON macro persistence.

See [architecture notes](docs/architecture.md) for extension points and known MVP limits.

## License

[MIT](LICENSE)
