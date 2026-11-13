# xcode-skills GUI

Native macOS app built with [Longbridge GPUI Kit](https://gpui-kit.com) for the same Xcode Skill Installations `xcode-skills` manages. It mirrors `xcode-skills manage`: Codex/Claude tabs, skill states, enable/disable, uninstall with confirmation.

## Requirements

- macOS 15 or later with Xcode (Metal rendering + SDK; Xcode 26 needs the Metal toolchain component: `xcodebuild -downloadComponent MetalToolchain`)
- Rust stable (`rustup`). `cargo` lives in `~/.cargo/bin`; `~/.zprofile` sources `~/.cargo/env`, so open a new terminal after installing.
- No Node/npm needed to build the app; the CLI stays TypeScript

## Build and run

```sh
cd gui
cargo run --locked
```

First build compiles GPUI Kit and its dependencies (several minutes). Later builds are incremental.

`gui/.cargo/config.toml` sets `DEVELOPER_DIR` to the standard Xcode path unless your shell already provides one. If your build fails with `xcrun: error: unable to find utility "metal"`, either run `xcodebuild -downloadComponent MetalToolchain` (Xcode 26) or export `DEVELOPER_DIR=/path/to/Xcode.app/Contents/Developer`.

Point the app at fixture roots instead of your real Xcode folders:

```sh
cargo run --locked -- --codex-root /tmp/codex-root --claude-root /tmp/claude-root
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

## Framework

| | |
|---|---|
| UI | GPUI Kit `=0.9.1` |
| GPUI | `gpui-pre` `0.3.12` (Zed GPUI snapshot at `d2a4fb6`), provided by Kit |
| Docs | https://gpui-kit.com/docs/ |

Application code imports GPUI only through `gpui_kit` (`use gpui_kit::*;`) and uses Kit components (`Button`, `TabBar`, `Tag`, `StatusBar`) styled with `cx.theme()` tokens. There is no direct `gpui`/`gpui-pre` dependency and no GPUI Fast backend. `Cargo.lock` is committed; use `--locked`.

## Architecture

```text
src/main.rs    application(), init, window, key bindings, --codex-root/--claude-root args
src/model.rs   integration roots, state scan, toggle, uninstall (unit tested, no GPUI imports)
src/ui.rs      Entity<SkillManager>, actions, Kit components, confirm flow
```

- Startup is Kit's: `application().with_assets(assets::Assets).run(|cx| { init(cx); ... open_window(...) })`. `open_window` wraps the view in a Base `Root`, which owns overlays and theme providers.
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

Prefer a Kit component from `gpui_kit::component`; check the catalog at https://gpui-kit.com/component. When no component fits, compose a small element next to `state_tag` in `src/ui.rs` and style it with theme tokens:

```rust
fn my_badge(label: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded(cx.theme().radius)
        .bg(cx.theme().secondary)
        .child(label.into())
}
```

Use `cx.theme()` colors only; no hard-coded palette. Interactive elements need `.id(...)` before `.on_click`/`.hover`.

## Add a feature

1. Filesystem behavior goes in `model.rs` with a test using a temp-dir fixture.
2. Add an action with `gpui_kit::actions!(...)` in `ui.rs`, bound in `main.rs` with a `KeyBinding` under the `"Skills"` context.
3. Handle it in `SkillManager`, call `reload()` and `cx.notify()` after mutations.
4. Surface the result in the status line (`self.status`).

## Verify

```sh
cd gui
cargo fmt
cargo clippy --all-targets --locked
cargo test --locked
cargo run --locked -- --codex-root /tmp/codex-root --claude-root /tmp/claude-root
```

Tests use fake roots; they never touch `~/Library/Developer/Xcode`. The UI test mounts the real view in a headless Kit window, presses real keys, and asserts the filesystem changed.

## Packaging

The MVP runs via `cargo run`. To ship, produce a `.app` bundle (`Contents/MacOS/xcode-skills-gui`, `Info.plist` with `NSHighResolutionCapable` and `LSMinimumSystemVersion` 15.0), then codesign and notarize, and wrap it in a DMG. Do this only when distribution is actually needed.
