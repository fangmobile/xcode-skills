## Agent skills

### Issue tracker

Issues and PRDs are tracked as local markdown under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage labels use the default mattpocock/skills vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain docs layout. See `docs/agents/domain.md`.

# Rust + GPUI macOS desktop app (`gui/`)

`gui/` is a native macOS app for the same Xcode Skill Installations the CLI manages. It mirrors the `xcode-skills manage` TUI (Codex/Claude tabs, skill rows, enable/disable, uninstall with confirmation). It is a native-rendered app, never a packaged web app.

## Non-negotiables

- Rust is the implementation language; GPUI is the UI framework.
- No HTML, CSS, JavaScript, TypeScript, React, Vue, Svelte, Electron, Tauri, or WebView; no embedded browser engine.
- Use native macOS capabilities where GPUI lacks them.
- Optimize for performance, low memory, fast startup, responsiveness.
- Keep the architecture modular and avoid unnecessary dependencies.

## GPUI rules

GPUI is pre-1.0 and breaks between releases.

1. Before implementing, inspect the pinned crate source (`~/.cargo/registry/src/*/gpui-<version>/src` and `examples/`), the release notes, and https://github.com/zed-industries/zed.
2. Verify every API against the pinned version. Never invent GPUI APIs or copy outdated examples.
3. Pin the dependency exactly (`gpui = "=0.2.2"` today). Bump deliberately and re-verify the build.
4. Keep GPUI code out of domain logic: `ui.rs` may call `model.rs`; `model.rs` must never import `gpui`.

## Stack

| Layer | Choice |
|---|---|
| Language | Rust stable, edition 2024 |
| UI | GPUI (pinned) |
| Rendering | GPUI Metal backend on macOS |
| Async | GPUI executor (`cx.spawn`); Tokio only when a dependency forces it |
| Serialization | `serde_json` for `skills-lock.json`; `serde` derives only when needed |
| Errors | `Result<T, std::io::Error>` for recoverable filesystem errors; avoid `anyhow`/`thiserror` until an error type earns it |
| Logging | `tracing` only when diagnostics are actually needed |
| Persistence | none — the filesystem is the source of truth, same as the CLI |
| Tests | `cargo test` (unit tests in `model.rs` with temp-dir fixtures) |
| Build | Cargo |
| Distribution | macOS `.app` bundle + DMG, signed/notarized, once shipping is in scope |

Do not add dependencies just because they are listed here.

## Architecture

```text
gui/
├── Cargo.toml
├── src/main.rs    app init, window, key bindings, CLI arg parsing
├── src/model.rs   integration roots, filesystem scan, toggle, uninstall (+ tests)
├── src/ui.rs      GPUI views, actions, confirm dialog, layout
└── src/theme.rs   dark/light palette derived from window appearance
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

`Entity<SkillManager>` owns all UI state (`roots`, per-tab `skills`, `active_tab`, per-tab selection, pending uninstall, status). Keyboard actions and clicks mutate that entity through `cx.listener(...)`, call `model`, then `reload()` and `cx.notify()`. No global mutable state, no watcher threads.

## UI/UX

- Consistent spacing and typography; system font; monospaced path text.
- Keyboard-first: keep the TUI bindings (`Tab` switch, `↑↓`/`j`/`k` move, `Space`/`Enter` toggle, `u` uninstall, `y`/`n` confirm, `⌘R` refresh, `⌘Q` quit).
- Dark and light themes; resizable windows; native titlebar behavior.
- Reusable components (`chip`, `state_pill`, `divider`) instead of ad-hoc styling.
- Design for desktop workflows; do not recreate browser UI patterns.

## Performance

- Avoid unnecessary clones/allocations inside `render`; reuse `&self` data and `SharedString`.
- Never block the UI thread. Current operations are local `readdir`/`rename`/`remove_dir_all`; anything slower (resolution, network) must go through `cx.spawn` and report back.
- No polling. Refresh on demand or after an action.
- Profile before optimizing.

## Code quality

- Rust idioms; explicit types at module boundaries; `Result` for recoverable errors.
- No unnecessary `unwrap()`/`expect()` outside startup.
- Small focused modules; tests for business logic in `model.rs`.
- Run `cargo fmt`, `cargo check`, `cargo clippy` in `gui/` before finishing. Do not suppress warnings without justification.

## Workflow

1. **Research** — confirm GPUI version/APIs and macOS build prerequisites (Xcode + Metal; `gui/.cargo/config.toml` defaults `DEVELOPER_DIR` to the standard Xcode path, and Xcode 26 needs `xcodebuild -downloadComponent MetalToolchain` for the `metal` compiler).
2. **Design** — module responsibilities, state ownership, how UI events reach `model`.
3. **Implement** — minimal working app: window, layout, components, keys, theme, clean shutdown.
4. **Verify** — `cargo test`; launch against fixture roots with `--codex-root`/`--claude-root`; confirm no UI-thread blocking and no web tech.
5. **Document** — update `gui/README.md` and this file when the contract changes.

Key bindings live in `main.rs`; actions and the `"Skills"` key context live in `ui.rs`.
