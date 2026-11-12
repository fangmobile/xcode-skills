mod model;
mod theme;
mod ui;

use std::path::PathBuf;
use std::process::ExitCode;

use gpui::prelude::*;
use gpui::{App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

use crate::model::RootOverrides;
use crate::ui::SkillManager;

fn main() -> ExitCode {
    let overrides = match parse_args(std::env::args().skip(1)) {
        Ok(overrides) => overrides,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    Application::new().run(move |cx: &mut App| {
        cx.bind_keys(ui::key_bindings());

        let bounds = Bounds::centered(None, size(px(880.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("xcode-skills".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(640.0), px(420.0))),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| SkillManager::new(overrides, cx));
                window.focus(view.read(cx).focus_handle());
                view
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });

    ExitCode::SUCCESS
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<RootOverrides, String> {
    let mut overrides = RootOverrides::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--codex-root" => {
                overrides.codex = Some(take_path(&mut args, "--codex-root")?);
            }
            "--claude-root" => {
                overrides.claude = Some(take_path(&mut args, "--claude-root")?);
            }
            "-h" | "--help" => {
                return Err(
                    "usage: xcode-skills-gui [--codex-root PATH] [--claude-root PATH]".to_string(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(overrides)
}

fn take_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} needs a path"))
}
