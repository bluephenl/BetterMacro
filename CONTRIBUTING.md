# Contributing

Please keep the macOS 12.6 deployment floor intact. New platform APIs need an availability check and a Monterey fallback. Do not add telemetry or cloud dependencies.

Before opening a pull request run:

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Automation changes need a manual macOS test: record clicks, normal typing, a Command shortcut, waiting, scrolling; replay it; verify Stop and `⌃⌥Esc`; save and reopen the macro.
