use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, FocusHandle, FontWeight, Render, SharedString,
    Stateful, Window, actions, div, px, white,
};

use crate::model::{self, IntegrationRoot, RootOverrides, Skill, SkillState};
use crate::theme::Palette;

actions!(
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

pub fn key_bindings() -> Vec<gpui::KeyBinding> {
    use gpui::KeyBinding;

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
    focus_handle: FocusHandle,
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
    pub fn new(overrides: RootOverrides, cx: &mut Context<Self>) -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let roots = model::resolve_roots(&home, &overrides);
        let tabs = roots
            .iter()
            .map(|root| model::scan(root).unwrap_or_default())
            .collect();

        Self {
            focus_handle: cx.focus_handle(),
            selected: vec![0; roots.len()],
            roots,
            tabs,
            active_tab: 0,
            pending: None,
            status: None,
        }
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
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

    fn select(&mut self, tab: usize, index: usize) {
        self.active_tab = tab;
        self.selected[tab] = index;
    }

    fn switch_tab(&mut self, cx: &mut Context<Self>) {
        if self.roots.is_empty() {
            return;
        }
        self.active_tab = (self.active_tab + 1) % self.roots.len();
        self.pending = None;
        cx.notify();
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

    fn toggle_selected(&mut self, cx: &mut Context<Self>) {
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

        self.select(tab, index);
        match model::toggle(&root, &identity) {
            Ok(status) => {
                self.status = Some(format!("{identity} → {status}").into());
                self.reload();
            }
            Err(error) => self.status = Some(format!("{identity}: {error}").into()),
        }
        cx.notify();
    }

    fn request_uninstall_selected(&mut self, cx: &mut Context<Self>) {
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
        self.select(tab, index);

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

    fn confirm_uninstall(&mut self, cx: &mut Context<Self>) {
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

    fn cancel_uninstall(&mut self, cx: &mut Context<Self>) {
        if self.pending.take().is_some() {
            cx.notify();
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload();
        self.status = Some("refreshed".into());
        cx.notify();
    }

    fn header(&self, palette: Palette, cx: &mut Context<Self>) -> Div {
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
                    .child(div().text_xs().text_color(palette.muted).child(location)),
            )
            .child(
                chip(SharedString::from("refresh"), "Refresh", palette)
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| this.refresh(cx))),
            )
    }

    fn tab_bar(&self, palette: Palette, cx: &mut Context<Self>) -> Div {
        div().flex().items_center().gap_2().px_4().py_3().children(
            self.roots.iter().enumerate().map(|(tab, root)| {
                let active = tab == self.active_tab;
                let count = self.tabs[tab].len();
                div()
                    .id(SharedString::from(format!("tab-{tab}")))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .when(active, |element| {
                        element.bg(palette.panel).text_color(palette.text)
                    })
                    .when(!active, |element| {
                        element
                            .text_color(palette.muted)
                            .hover(|style| style.text_color(palette.text))
                    })
                    .child(format!("{}  {count}", root.id.display()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.select(tab, this.selected[tab]);
                        this.pending = None;
                        cx.notify();
                    }))
            }),
        )
    }

    fn body(&self, palette: Palette, cx: &mut Context<Self>) -> AnyElement {
        if let Some(pending) = &self.pending {
            return self.confirm_card(pending, palette, cx).into_any_element();
        }

        let root = &self.roots[self.active_tab];
        if !root.activated {
            return empty_state(
                format!("{} is not activated in Xcode.", root.id.display()),
                palette,
            );
        }

        let skills = &self.tabs[self.active_tab];
        if skills.is_empty() {
            return empty_state("No skills installed.".to_string(), palette);
        }

        let selected = self.selected[self.active_tab];
        div()
            .id("skill-list")
            .flex_1()
            .overflow_scroll()
            .px_4()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(skills.iter().enumerate().map(|(index, skill)| {
                self.skill_row(
                    self.active_tab,
                    index,
                    skill,
                    index == selected,
                    palette,
                    cx,
                )
            }))
            .into_any_element()
    }

    fn skill_row(
        &self,
        tab: usize,
        index: usize,
        skill: &Skill,
        selected: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut meta: Vec<String> = Vec::new();
        if let Some(source) = skill.source_label() {
            meta.push(source);
        }
        if let Some(hash) = skill.short_hash() {
            meta.push(hash.to_string());
        }

        let mut toggle = match skill.state {
            SkillState::Enabled => chip(
                SharedString::from(format!("toggle-{tab}-{index}")),
                "Disable",
                palette,
            ),
            SkillState::Disabled => chip(
                SharedString::from(format!("toggle-{tab}-{index}")),
                "Enable",
                palette,
            ),
            SkillState::Conflict | SkillState::Suspicious => chip(
                SharedString::from(format!("toggle-{tab}-{index}")),
                "—",
                palette,
            ),
        };
        if matches!(skill.state, SkillState::Enabled | SkillState::Disabled) {
            toggle =
                toggle.on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.toggle_at(tab, index, cx)
                }));
        }

        div()
            .id(SharedString::from(format!("skill-{tab}-{index}")))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .when(selected, |element| element.bg(palette.selected))
            .when(!selected, |element| {
                element.hover(|style| style.bg(palette.panel_hover))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.select(tab, index);
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
                            .text_color(palette.muted)
                            .child(meta.join("  ")),
                    ),
            )
            .child(state_pill(skill.state, palette))
            .child(toggle)
            .child(
                chip(
                    SharedString::from(format!("uninstall-{tab}-{index}")),
                    "Uninstall",
                    palette,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.request_uninstall_at(tab, index, cx)
                })),
            )
            .into_any_element()
    }

    fn confirm_card(
        &self,
        pending: &PendingUninstall,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let integration = self.roots[pending.tab].id.display();
        div().flex_1().p_4().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .rounded_md()
                .bg(palette.panel)
                .border_1()
                .border_color(palette.danger)
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Uninstall {}?", pending.identity)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child(format!("From {integration}. These folders are deleted:")),
                )
                .children(pending.paths.iter().map(|path| {
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child(path.to_string_lossy().into_owned())
                }))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            chip(SharedString::from("cancel-uninstall"), "Cancel", palette)
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.cancel_uninstall(cx)
                                })),
                        )
                        .child(
                            div()
                                .id("confirm-uninstall")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(palette.danger)
                                .text_xs()
                                .text_color(white())
                                .cursor_pointer()
                                .child("Uninstall")
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.confirm_uninstall(cx)
                                })),
                        ),
                ),
        )
    }

    fn footer(&self, palette: Palette) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_4()
            .py_3()
            .text_xs()
            .text_color(palette.muted)
            .child("Tab switch · ↑↓/j/k move · Space toggle · u uninstall · ⌘R refresh · ⌘Q quit")
            .child(
                div()
                    .text_color(palette.accent)
                    .child(self.status.clone().unwrap_or_default()),
            )
    }
}

impl Render for SkillManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::from_appearance(window.appearance());

        div()
            .key_context("Skills")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(palette.bg)
            .text_color(palette.text)
            .on_action(cx.listener(|this, _: &SwitchTab, _window, cx| this.switch_tab(cx)))
            .on_action(cx.listener(|this, _: &SelectNext, _window, cx| this.move_selection(1, cx)))
            .on_action(cx.listener(|this, _: &SelectPrev, _window, cx| this.move_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &ToggleSkill, _window, cx| this.toggle_selected(cx)))
            .on_action(cx.listener(|this, _: &RequestUninstall, _window, cx| {
                this.request_uninstall_selected(cx)
            }))
            .on_action(
                cx.listener(|this, _: &ConfirmUninstall, _window, cx| this.confirm_uninstall(cx)),
            )
            .on_action(
                cx.listener(|this, _: &CancelUninstall, _window, cx| this.cancel_uninstall(cx)),
            )
            .on_action(cx.listener(|this, _: &Refresh, _window, cx| this.refresh(cx)))
            .on_action(cx.listener(|_this, _: &Quit, _window, cx| cx.quit()))
            .child(self.header(palette, cx))
            .child(divider(palette))
            .child(self.tab_bar(palette, cx))
            .child(self.body(palette, cx))
            .child(divider(palette))
            .child(self.footer(palette))
    }
}

fn divider(palette: Palette) -> Div {
    div().h(px(1.0)).w_full().bg(palette.border)
}

fn empty_state(message: String, palette: Palette) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(palette.muted)
        .child(message)
        .into_any_element()
}

fn state_pill(state: SkillState, palette: Palette) -> Div {
    let color = match state {
        SkillState::Enabled => palette.success,
        SkillState::Disabled => palette.muted,
        SkillState::Conflict => palette.warning,
        SkillState::Suspicious => palette.danger,
    };

    div()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(color)
        .text_xs()
        .text_color(color)
        .child(state.label())
}

fn chip(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    palette: Palette,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(palette.border)
        .bg(palette.panel)
        .text_xs()
        .text_color(palette.muted)
        .cursor_pointer()
        .hover(|style| style.bg(palette.panel_hover).text_color(palette.text))
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gpui::{Entity, TestAppContext, VisualTestContext};

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

    fn states(
        cx: &mut VisualTestContext,
        view: &Entity<SkillManager>,
    ) -> Vec<(String, SkillState)> {
        cx.update(|_window, cx| {
            view.read(cx).tabs[0]
                .iter()
                .map(|skill| (skill.identity.clone(), skill.state))
                .collect()
        })
    }

    #[gpui::test]
    fn keyboard_toggles_and_uninstalls_skills(cx: &mut TestAppContext) {
        let (dir, overrides) = fixture();
        let codex = overrides.codex.clone().unwrap();
        cx.update(|cx| cx.bind_keys(key_bindings()));

        let (view, cx) = cx.add_window_view(|window, cx| {
            let view = SkillManager::new(overrides.clone(), cx);
            window.focus(view.focus_handle());
            view
        });

        assert_eq!(
            states(cx, &view),
            vec![
                ("alpha".to_string(), SkillState::Enabled),
                ("beta".to_string(), SkillState::Disabled),
            ]
        );

        cx.simulate_keystrokes("space");
        assert!(
            codex
                .join(".xcode-skills/disabled/alpha/SKILL.md")
                .is_file()
        );
        assert!(!codex.join("skills/alpha").exists());

        cx.simulate_keystrokes("down space");
        assert!(codex.join("skills/beta/SKILL.md").is_file());
        assert!(!codex.join(".xcode-skills/disabled/beta").exists());

        cx.simulate_keystrokes("u n");
        assert!(codex.join("skills/beta").exists());

        cx.simulate_keystrokes("u y");
        assert!(!codex.join("skills/beta").exists());
        assert!(!codex.join(".xcode-skills/disabled/beta").exists());

        cx.simulate_keystrokes("tab");
        cx.update(|_window, cx| assert_eq!(view.read(cx).active_tab, 1));

        let _ = fs::remove_dir_all(&dir);
    }
}
