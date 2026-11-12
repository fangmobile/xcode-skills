# xcode-skills GUI

Native macOS app (Rust + [GPUI](https://gpui.rs)) for the same Xcode Skill Installations `xcode-skills` manages. It mirrors `xcode-skills manage`: Codex/Claude tabs, skill states, enable/disable, uninstall with confirmation.

## Requirements

- macOS with Xcode (Metal rendering + SDK; the Metal toolchain component is needed on Xcode 26: `xcodebuild -downloadComponent MetalToolchain`)
- Rust stable (`rustup`). `cargo` lives in `~/.cargo/bin`; `~/.zprofile` sources `~/.cargo/env`, so open a new terminal after installing.
- No Node/npm needed to build the app; the CLI stays TypeScript

## Build and run

```sh
cd gui
cargo run
```

First build compiles GPUI and its dependencies (several minutes). Later builds are incremental.

`gui/.cargo/config.toml` sets `DEVELOPER_DIR` to the standard Xcode path unless your shell already provides one. If your build fails with `xcrun: error: unable to find utility "metal"`, either run `xcodebuild -downloadComponent MetalToolchain` (Xcode 26) or export `DEVELOPER_DIR=/path/to/Xcode.app/Contents/Developer`.

Point the app at fixture roots instead of your real Xcode folders:

```sh
cargo run -- --codex-root /tmp/codex-root --claude-root /tmp/claude-root
```

## Controls

| Key | Action |
|---|---|
| `Tab` | switch Codex/Claude tab |
| `↑` / `↓` / `k` / `j` | move selection |
| `Space` / `Enter` | enable/disable selected skill |
| `u` | uninstall selected skill (then `y` confirm / `n` escape) |
| `⌘R` | rescan filesystem |
| `⌘Q` | quit |

Every action is also available with the mouse.

## Architecture

```text
src/main.rs    app init, window, key bindings, --codex-root/--claude-root args
src/model.rs   integration roots, state scan, toggle, uninstall (unit tested)
src/ui.rs      Entity<SkillManager>, actions, layout, confirm dialog
src/theme.rs   dark/light palette from window appearance
```

- `model.rs` has no GPUI imports; it is the same contract as the TypeScript CLI (`src/integrations.ts`, `src/filesystem-state.ts`, `src/lifecycle.ts`, `src/uninstall.ts`).
- `ui.rs` owns all view state in one `Entity<SkillManager>`: roots, per-tab skills, active tab, selection, pending uninstall, status text. Actions mutate it through `cx.listener`, call `model`, reload, then `cx.notify()`.
- The filesystem is the source of truth. There is no database and nothing is cached across launches.

State model:

```text
<integration-root>/skills/<identity>/                     enabled
<integration-root>/.xcode-skills/disabled/<identity>/      disabled
both exist                                                 conflict
folder without SKILL.md                                    suspicious
```

## Add a UI component

Define a small function returning an element next to `chip`/`state_pill` in `src/ui.rs`:

```rust
fn my_badge(label: impl Into<SharedString>, palette: Palette) -> Div {
    div().px_2().py_1().rounded_md().bg(palette.panel).child(label.into())
}
```

Use `palette` colors only, from `theme.rs`. Interactive elements need `.id(...)` before `.on_click`/`.hover`.

## Add a feature

1. Filesystem behavior goes in `model.rs` with a test using a temp-dir fixture.
2. Add an action to `actions!(...)` in `ui.rs`, bound in `main.rs` with a `KeyBinding` under the `"Skills"` context.
3. Handle it in `SkillManager`, call `reload()` and `cx.notify()` after mutations.
4. Surface the result in the status line (`self.status`).

## Verify

```sh
cd gui
cargo fmt
cargo clippy
cargo test
cargo run -- --codex-root /tmp/codex-root --claude-root /tmp/claude-root
```

Tests use fake roots; they never touch `~/Library/Developer/Xcode`.

## Packaging

The MVP runs via `cargo run`. To ship, produce a `.app` bundle (`Contents/MacOS/xcode-skills-gui`, `Info.plist` with `NSHighResolutionCapable`), then codesign and notarize, and wrap it in a DMG. Do this only when distribution is actually needed.
