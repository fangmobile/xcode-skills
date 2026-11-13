# Use Longbridge GPUI Kit for the GUI

The `gui/` macOS app depends on `gpui-kit = "=0.9.1"` instead of Zed's `gpui` crate or a pinned Git revision of `zed-industries/zed`. Kit exposes GPUI through one facade (`use gpui_kit::*;`), supplies a published `gpui-pre = 0.3.12` snapshot of Zed GPUI at commit `d2a4fb6`, and adds GPUI Base and GPUI Component so application startup, theme tokens, and standard controls do not have to be built and versioned separately.

Consequences:

- `gui/Cargo.toml` lists one framework dependency; `gpui-pre*` crates are transitive and must not be overridden or depended on directly.
- `gui/Cargo.lock` is committed and builds run with `--locked`. Framework upgrades are deliberate: review the release, then re-run `cargo test --locked`.
- The default upstream GPUI backend is used. GPUI Fast (`gpui-kit`'s `gpui-fast` feature) is not enabled.
- Rendering requires Xcode's Metal toolchain; Xcode 26 ships it as a downloadable component (`xcodebuild -downloadComponent MetalToolchain`), and `gui/.cargo/config.toml` defaults `DEVELOPER_DIR` for command line builds.
