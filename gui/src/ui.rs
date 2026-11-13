use std::path::PathBuf;

use gpui_kit::base::Disableable;
use gpui_kit::component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    status_bar::StatusBar,
    tab::{Tab, TabBar},
    tag::{Tag, TagVariant},
};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Div, FontWeight, Render, SharedString, Window, div,
};

use crate::model::{self, IntegrationRoot, RootOverrides, Skill, SkillState};

gpui_kit::actions!(
    xcode_skills,
    [
        SwitchTab,
        SelectNext,
        SelectPrev,
        ToggleSkill,
        RequestUninstall,
        ConfirmUninstall,
        CancelUninstall,
        Refresh,
        Quit
    ]
);

pub fn key_bindings() -> Vec<gpui_kit::KeyBinding> {
    use gpui_kit::KeyBinding;

    vec![
        KeyBinding::new("tab", SwitchTab, Some("Skills")),
        KeyBinding::new("down", SelectNext, Some("Skills")),
        KeyBinding::new("j", SelectNext, Some("Skills")),
        KeyBinding::new("up", SelectPrev, Some("Skills")),
        KeyBinding::new("k", SelectPrev, Some("Skills")),
        KeyBinding::new("space", ToggleSkill, Some("Skills")),
        KeyBinding::new("enter", ToggleSkill, Some("Skills")),
        KeyBinding::new("u", RequestUninstall, Some("Skills")),
        KeyBinding::new("y", ConfirmUninstall, Some("Skills")),
        KeyBinding::new("n", CancelUninstall, Some("Skills")),
        KeyBinding::new("escape", CancelUninstall, Some("Skills")),
        KeyBinding::new("cmd-r", Refresh, Some("Skills")),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}

pub struct SkillManager {
    focus_handle: gpui_kit::FocusHandle,
    roots: Vec<IntegrationRoot>,
    tabs: Vec<Vec<Skill>>,
    active_tab: usize,
    selected: Vec<usize>,
    pending: Option<PendingUninstall>,
    status: Option<SharedString>,
}

struct PendingUninstall {
    tab: usize,
    identity: String,
    paths: Vec<PathBuf>,
}

impl SkillManager {
    pub fn new(overrides: RootOverrides, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let roots = model::resolve_roots(&home, &overrides);
        let tabs = roots
            .iter()
            .map(|root| model::scan(root).unwrap_or_default())
            .collect();
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            focus_handle,
            selected: vec![0; roots.len()],
            roots,
            tabs,
            active_tab: 0,
            pending: None,
            status: None,
        }
    }

    fn reload(&mut self) {
        self.tabs = self
            .roots
            .iter()
            .map(|root| model::scan(root).unwrap_or_default())
            .collect();
        for (skills, selected) in self.tabs.iter().zip(self.selected.iter_mut()) {
            *selected = (*selected).min(skills.len().saturating_sub(1));
        }
    }

    fn select(&mut self, tab: usize, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab = tab;
        self.selected[tab] = index;
        self.focus_handle.focus(window, cx);
    }

    fn select_tab(&mut self, tab: usize, window: &mut Window, cx: &mut Context<Self>) {
        if tab >= self.roots.len() {
            return;
        }
        self.active_tab = tab;
        self.pending = None;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn switch_tab(&mut self, _: &SwitchTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.roots.is_empty() {
            return;
        }
        let tab = (self.active_tab + 1) % self.roots.len();
        self.select_tab(tab, window, cx);
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.tabs[self.active_tab].len();
        if count == 0 {
            return;
        }
        let selected = self.selected[self.active_tab] as isize;
        self.selected[self.active_tab] = (selected + delta).rem_euclid(count as isize) as usize;
        cx.notify();
    }

    fn select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(1, cx);
    }

    fn select_prev(&mut self, _: &SelectPrev, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(-1, cx);
    }

    fn toggle_selected(&mut self, _: &ToggleSkill, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_at(self.active_tab, self.selected[self.active_tab], cx);
    }

    fn toggle_at(&mut self, tab: usize, index: usize, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        let Some(skill) = self.tabs.get(tab).and_then(|skills| skills.get(index)) else {
            return;
        };
        let identity = skill.identity.clone();
        let root = self.roots[tab].clone();

        self.active_tab = tab;
        self.selected[tab] = index;
        match model::toggle(&root, &identity) {
            Ok(status) => {
                self.status = Some(format!("{identity} → {status}").into());
                self.reload();
            }
            Err(error) => self.status = Some(format!("{identity}: {error}").into()),
        }
        cx.notify();
    }

    fn request_uninstall_selected(
        &mut self,
        _: &RequestUninstall,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_uninstall_at(self.active_tab, self.selected[self.active_tab], cx);
    }

    fn request_uninstall_at(&mut self, tab: usize, index: usize, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        let Some(skill) = self.tabs.get(tab).and_then(|skills| skills.get(index)) else {
            return;
        };
        let identity = skill.identity.clone();
        let paths = model::removal_targets(&self.roots[tab], &identity);
        self.active_tab = tab;
        self.selected[tab] = index;

        if paths.is_empty() {
            self.status = Some(format!("{identity}: nothing to remove").into());
        } else {
            self.pending = Some(PendingUninstall {
                tab,
                identity,
                paths,
            });
        }
        cx.notify();
    }

    fn confirm_uninstall(&mut self, _: &ConfirmUninstall, _: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        match model::remove(&pending.paths) {
            Ok(()) => {
                self.status = Some(format!("uninstalled {}", pending.identity).into());
                self.reload();
            }
            Err(error) => self.status = Some(format!("{}: {error}", pending.identity).into()),
        }
        cx.notify();
    }

    fn cancel_uninstall(&mut self, _: &CancelUninstall, _: &mut Window, cx: &mut Context<Self>) {
        if self.pending.take().is_some() {
            cx.notify();
        }
    }

    fn refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.reload();
        self.status = Some("refreshed".into());
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        let location = match self.roots.get(self.active_tab) {
            Some(root) => format!("{} — {}", root.id.display(), root.root.to_string_lossy()),
            None => String::new(),
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_4()
            .py_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Xcode Skills"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(location),
                    ),
            )
            .child(Button::new("refresh").small().label("Refresh").on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.refresh(&Refresh, window, cx)),
            ))
    }

    fn tab_bar(&self, cx: &mut Context<Self>) -> TabBar {
        TabBar::new("integration-tabs")
            .underline()
            .selected_index(self.active_tab)
            .children(self.roots.iter().enumerate().map(|(tab, root)| {
                Tab::new().label(format!("{}  {}", root.id.display(), self.tabs[tab].len()))
            }))
            .on_click(
                cx.listener(|this, tab: &usize, window, cx| this.select_tab(*tab, window, cx)),
            )
    }

    fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(pending) = &self.pending {
            return self.confirm_card(pending, cx).into_any_element();
        }

        let root = &self.roots[self.active_tab];
        if !root.activated {
            return empty_state(
                format!("{} is not activated in Xcode.", root.id.display()),
                cx,
            );
        }

        let skills = &self.tabs[self.active_tab];
        if skills.is_empty() {
            return empty_state("No skills installed.".to_string(), cx);
        }

        let selected = self.selected[self.active_tab];
        div()
            .id("skill-list")
            .flex_1()
            .overflow_y_scroll()
            .px_4()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(skills.iter().enumerate().map(|(index, skill)| {
                self.skill_row(self.active_tab, index, skill, index == selected, cx)
            }))
            .into_any_element()
    }

    fn skill_row(
        &self,
        tab: usize,
        index: usize,
        skill: &Skill,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (selected_bg, hover_bg) = (cx.theme().list_active, cx.theme().list_hover);
        let mut meta: Vec<String> = Vec::new();
        if let Some(source) = skill.source_label() {
            meta.push(source);
        }
        if let Some(hash) = skill.short_hash() {
            meta.push(hash.to_string());
        }
        let toggleable = matches!(skill.state, SkillState::Enabled | SkillState::Disabled);

        div()
            .id(SharedString::from(format!("skill-{tab}-{index}")))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_2()
            .rounded(cx.theme().radius)
            .when(selected, |element| element.bg(selected_bg))
            .when(!selected, |element| {
                element.hover(|style| style.bg(hover_bg))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.select(tab, index, window, cx);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(skill.identity.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(meta.join("  ")),
                    ),
            )
            .child(state_tag(skill.state))
            .child(
                Button::new(SharedString::from(format!("toggle-{tab}-{index}")))
                    .small()
                    .label(match skill.state {
                        SkillState::Enabled => "Disable",
                        SkillState::Disabled => "Enable",
                        _ => "—",
                    })
                    .disabled(!toggleable)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_at(tab, index, cx)
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("uninstall-{tab}-{index}")))
                    .small()
                    .danger()
                    .label("Uninstall")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.request_uninstall_at(tab, index, cx)
                    })),
            )
            .into_any_element()
    }

    fn confirm_card(&self, pending: &PendingUninstall, cx: &mut Context<Self>) -> Div {
        let integration = self.roots[pending.tab].id.display();
        div().flex_1().p_4().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().popover)
                .border_1()
                .border_color(cx.theme().danger)
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Uninstall {}?", pending.identity)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("From {integration}. These folders are deleted:")),
                )
                .children(pending.paths.iter().map(|path| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(path.to_string_lossy().into_owned())
                }))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Button::new("cancel-uninstall")
                                .small()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.cancel_uninstall(&CancelUninstall, window, cx)
                                })),
                        )
                        .child(
                            Button::new("confirm-uninstall")
                                .small()
                                .danger()
                                .label("Uninstall")
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.confirm_uninstall(&ConfirmUninstall, window, cx)
                                })),
                        ),
                ),
        )
    }

    fn footer(&self, cx: &mut Context<Self>) -> StatusBar {
        StatusBar::new()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .left("Tab switch · ↑↓/j/k move · Space toggle · u uninstall · ⌘R refresh · ⌘Q quit")
            .right(
                div()
                    .text_color(cx.theme().primary)
                    .child(self.status.clone().unwrap_or_default()),
            )
    }
}

impl Render for SkillManager {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Skills")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(Self::switch_tab))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .on_action(cx.listener(Self::toggle_selected))
            .on_action(cx.listener(Self::request_uninstall_selected))
            .on_action(cx.listener(Self::confirm_uninstall))
            .on_action(cx.listener(Self::cancel_uninstall))
            .on_action(cx.listener(Self::refresh))
            .on_action(cx.listener(Self::quit))
            .child(self.header(cx))
            .child(self.tab_bar(cx))
            .child(self.body(cx))
            .child(self.footer(cx))
    }
}

fn empty_state(message: String, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(message)
        .into_any_element()
}

fn state_tag(state: SkillState) -> Tag {
    let variant = match state {
        SkillState::Enabled => TagVariant::Success,
        SkillState::Disabled => TagVariant::Secondary,
        SkillState::Conflict => TagVariant::Warning,
        SkillState::Suspicious => TagVariant::Danger,
    };

    Tag::new()
        .with_variant(variant)
        .outline()
        .small()
        .child(state.label())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds, WindowHandle,
        WindowOptions, base::Root, px, size,
    };

    use super::*;
    use crate::model::RootOverrides;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn write_skill(root: &Path, kind: &str, identity: &str) {
        let folder = root.join(kind).join(identity);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("SKILL.md"), "# skill").unwrap();
    }

    fn fixture() -> (PathBuf, RootOverrides) {
        let dir = std::env::temp_dir().join(format!(
            "xcode-skills-gui-ui-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        let codex = dir.join("codex");
        write_skill(&codex, "skills", "alpha");
        write_skill(&codex, ".xcode-skills/disabled", "beta");
        let overrides = RootOverrides {
            codex: Some(codex),
            claude: Some(dir.join("missing")),
        };
        (dir, overrides)
    }

    fn open_test_window(
        cx: &mut TestAppContext,
        overrides: RootOverrides,
    ) -> (WindowHandle<Root>, Entity<SkillManager>) {
        cx.update(|cx| {
            let bounds = Bounds {
                origin: Point::default(),
                size: size(px(880.), px(600.)),
            };
            let (window, view) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| SkillManager::new(overrides, window, cx)),
            )
            .expect("open test window");
            (window.downcast::<Root>().expect("Base Root"), view)
        })
    }

    #[gpui_kit::test]
    fn keyboard_toggles_and_uninstalls_skills(cx: &mut TestAppContext) {
        let (dir, overrides) = fixture();
        let codex = overrides.codex.clone().unwrap();
        cx.update(gpui_kit::init);
        cx.update(|cx| cx.bind_keys(key_bindings()));

        let (window, view) = open_test_window(cx, overrides);

        cx.update_window(window.into(), |_, window, cx| {
            window.render_frame(cx);

            window.press("space", cx);
            assert!(
                codex
                    .join(".xcode-skills/disabled/alpha/SKILL.md")
                    .is_file()
            );
            assert!(!codex.join("skills/alpha").exists());

            window.press("down", cx);
            window.press("space", cx);
            assert!(codex.join("skills/beta/SKILL.md").is_file());
            assert!(!codex.join(".xcode-skills/disabled/beta").exists());

            window.press("u", cx);
            window.press("n", cx);
            assert!(codex.join("skills/beta").exists());

            window.press("u", cx);
            window.press("y", cx);
            assert!(!codex.join("skills/beta").exists());
            assert!(!codex.join(".xcode-skills/disabled/beta").exists());

            window.press("tab", cx);
        })
        .unwrap();

        cx.update(|cx| assert_eq!(view.read(cx).active_tab, 1));

        let _ = fs::remove_dir_all(&dir);
    }
}
