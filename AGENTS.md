## Agent skills

### Issue tracker

Issues and PRDs are tracked as local markdown under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage labels use the default mattpocock/skills vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain docs layout. See `docs/agents/domain.md`.

# Rust + GPUI Kit macOS desktop app (`gui/`)

`gui/` is a native macOS app for the same Xcode Skill Installations the CLI manages. It mirrors the `xcode-skills manage` TUI (Codex/Claude tabs, skill rows, enable/disable, uninstall with confirmation). It is a native-rendered app, never a packaged web app.

## Selected framework

| | |
|---|---|
| Framework | Longbridge GPUI Kit, `gpui-kit = "=0.9.1"` (one application dependency) |
| Underlying GPUI | `gpui-pre = 0.3.12`, a published snapshot of Zed GPUI at commit `d2a4fb6`, brought in by Kit |
| Docs | https://gpui-kit.com/docs/ (written for 0.9.1) |
| Source | https://github.com/longbridge/gpui-kit |

- Import GPUI APIs through `gpui_kit` (`use gpui_kit::*;`). Never add direct `gpui-pre*` dependencies, alias them, or override Kit's pinned GPUI crates.
- Never mix GPUI distributions or forks.
- Commit `Cargo.lock`; build and test with `cargo build --locked` / `cargo test --locked`.
- Do not upgrade GPUI Kit or its underlying GPUI snapshot on your own. Review the release, then re-run the full build and test suite.
- Target macOS 15 or later with the upstream GPUI Metal backend.

## Non-negotiables

- Rust is the implementation language; GPUI Kit is the UI framework.
- No HTML, CSS, JavaScript, TypeScript, React, Vue, Svelte, Electron, Tauri, or WebView; no embedded browser engine; no JavaScript extension runtime.
- Do not enable the GPUI Fast backend (`gpui-kit`'s `gpui-fast` feature); use Kit's default upstream GPUI backend.
- Use native macOS capabilities where GPUI lacks them.
- Optimize for performance, low memory, fast startup, responsiveness.
- Keep the architecture modular and avoid unnecessary dependencies.

## GPUI Kit rules

GPUI is pre-1.0 and breaks between releases.

1. Before implementing anything, read https://gpui-kit.com/docs/ and inspect the pinned crate sources: `~/.cargo/registry/src/*/gpui-kit-0.9.1`, `gpui-component-0.9.1`, `gpui-base-0.9.1`, plus their `tests/` and `gpui-pre-0.3.12/examples/`.
2. Verify every API against version 0.9.1. Never invent GPUI APIs or copy examples written for another GPUI version.
3. Use Kit's startup pattern: `gpui_kit::application().with_assets(assets::Assets).run(...)`, `gpui_kit::init(cx)`, then `gpui_kit::open_window(...)`. `open_window` wraps the view in a Base `Root`; return application content, not another `Root`.
4. Use Kit components where appropriate (`component::button`, `tab`, `tag`, `status_bar`, ...) and GPUI Base infrastructure (focus handles, actions, key contexts). Build a custom element only when no component covers the case.
5. Theme through `cx.theme()` tokens (`ActiveTheme`). No hard-coded palettes and no second theme module.
6. Keep GPUI code out of domain logic: `ui.rs` may call `model.rs`; `model.rs` must never import `gpui`/`gpui_kit`.

## Stack

| Layer | Choice |
|---|---|
| Language | Rust stable, edition 2024 |
| UI | GPUI Kit `=0.9.1` (`gpui-pre` `0.3.12`) |
| Rendering | upstream GPUI Metal backend on macOS (not GPUI Fast) |
| Components | `gpui_kit::component` (GPUI Component) + `gpui_kit::base` |
| Async | GPUI executor (`cx.spawn`); Tokio only when a dependency forces it |
| Serialization | `serde_json` for `skills-lock.json`; `serde` derives only when needed |
| Errors | `Result<T, std::io::Error>` for recoverable filesystem errors; avoid `anyhow`/`thiserror` until an error type earns it |
| Logging | `tracing` only when diagnostics are actually needed |
| Persistence | none — the filesystem is the source of truth, same as the CLI |
| Tests | `cargo test --locked`; unit tests in `model.rs`, `#[gpui_kit::test]` for UI integration |
| Build | `cargo build --locked` |
| Distribution | macOS `.app` bundle + DMG, signed/notarized, once shipping is in scope |

Do not add dependencies just because they are listed here.

## Architecture

```text
gui/
├── Cargo.toml
├── Cargo.lock
├── src/main.rs    app init, window, key bindings, CLI arg parsing
├── src/model.rs   integration roots, filesystem scan, toggle, uninstall (+ tests)
└── src/ui.rs      GPUI Kit views, actions, components, confirm flow
```

One crate is deliberate: three modules cover the whole surface. Split into `crates/app-core`, `crates/app-ui`, etc. only when a module earns separate compilation or reuse. `main.rs` wires; `ui.rs` renders; `model.rs` decides.

### Model contract (keep in lockstep with `src/`)

- Roots: `~/Library/Developer/Xcode/CodingAssistant/{codex,ClaudeAgentConfig}`; `activated` means the root exists.
- Enabled: `<root>/skills/<identity>/`; disabled: `<root>/.xcode-skills/disabled/<identity>/`.
- State: `enabled`, `disabled`, `conflict` (both copies), `suspicious` (no `SKILL.md`).
- Toggle is a folder rename; uninstall deletes both copies; there is no manifest.
- Provenance comes from `<skill>/skills-lock.json` (`source`, `sourceType`, `computedHash`).
- State changes only after a successful filesystem operation; failures surface in the status line.

### State and events

`Entity<SkillManager>` owns all UI state (`roots`, per-tab `skills`, `active_tab`, per-tab selection, pending uninstall, status). The view keeps the focused `FocusHandle` and a `"Skills"` key context; actions and clicks mutate the entity through `cx.listener(...)`, call `model`, then `reload()` and `cx.notify()`. No global mutable state, no watcher threads.

## UI/UX

- Use Kit theme tokens, components, and spacing/typography; the theme follows the system light/dark appearance.
- Keyboard-first: keep the TUI bindings (`Tab` switch, `↑↓`/`j`/`k` move, `Space`/`Enter` toggle, `u` uninstall, `y`/`n` confirm, `⌘R` refresh, `⌘Q` quit).
- Focus stays on the view's own handle; clicking a row returns focus so shortcuts keep working.
- Resizable windows, native titlebar behavior, accessible labels on interactive elements.
- Design for desktop workflows; do not recreate browser UI patterns.

## Performance

- Avoid unnecessary clones/allocations inside `render`; reuse `&self` data and `SharedString`.
- Never block the UI thread. Current operations are local `readdir`/`rename`/`remove_dir_all`; anything slower (resolution, network) must go through `cx.spawn` and report back.
- No polling. Refresh on demand or after an action.
- Keep the documented `[profile.dev.package]` overrides so framework layers stay optimized in dev builds.
- Profile before optimizing.

## Code quality

- Rust idioms; explicit types at module boundaries; `Result` for recoverable errors.
- No unnecessary `unwrap()`/`expect()` outside startup.
- Small focused modules; tests for business logic in `model.rs`.
- Run `cargo fmt`, `cargo clippy --locked`, `cargo check --locked`, `cargo test --locked` in `gui/` before finishing. Do not suppress warnings without justification.

## Workflow

1. **Research** — read the GPUI Kit docs for the pinned version and confirm APIs in its crate source. Build prerequisites: Xcode + Metal; `gui/.cargo/config.toml` defaults `DEVELOPER_DIR` to the standard Xcode path, and Xcode 26 needs `xcodebuild -downloadComponent MetalToolchain` for the `metal` compiler.
2. **Design** — module responsibilities, state ownership, how UI actions reach `model`.
3. **Implement** — window, layout, Kit components, keys, theme tokens, clean shutdown.
4. **Verify** — `cargo test --locked`; launch against fixture roots with `--codex-root`/`--claude-root`; confirm no UI-thread blocking and no web tech.
5. **Document** — update `gui/README.md` and this file when the contract changes.

Key bindings live in `main.rs`; actions and the `"Skills"` key context live in `ui.rs`.
