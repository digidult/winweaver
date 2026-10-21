//! The navigation row: the history, the breadcrumb, the star and the search of the tab.

use super::*;

impl App {
    /// The breadcrumb of the active tab: a label and what a click on it does.
    fn nav_crumbs(&self) -> Vec<(String, Option<Act>)> {
        match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => self
                .crumbs()
                .into_iter()
                .map(|(label, sel)| {
                    let act = match sel {
                        Sel::Rule(_) => Act::Profile(ProfileAct::SelectIn(sel, doc.focus.clone())),
                        _ => Act::Profile(ProfileAct::Select(sel)),
                    };
                    (label, Some(act))
                })
                .collect(),
            Some(Doc::Catalog(c)) => {
                let Some(file) = c.catalog(&self.imports) else {
                    return Vec::new();
                };
                let mut out = vec![(
                    file.name.clone(),
                    Some(Act::Catalog(CatalogAct::Select(None))),
                )];
                let lang = crate::i18n::current();
                let rule = c
                    .sel
                    .as_deref()
                    .and_then(|id| file.rules.iter().find(|r| r.id == id));
                let mut group = match rule {
                    Some(r) => Some(r.group.clone()),
                    None => c.group.clone(),
                };
                let mut chain = Vec::new();
                while let Some(g) = group {
                    let Some(found) = file.groups.iter().find(|x| x.id == g) else {
                        break;
                    };
                    let title = crate::catalogview::group_title(file, &lang, found).to_owned();
                    chain.push((
                        title,
                        Some(Act::Catalog(CatalogAct::Select(Some(TreeKey::Group(g))))),
                    ));
                    group = found.parent.clone();
                }
                out.extend(chain.into_iter().rev());
                if let Some(rule) = rule {
                    let (title, _) = crate::catalogview::rule_texts(file, &lang, rule);
                    out.push((title.to_owned(), None));
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// The navigation row of the active tab: the history, the breadcrumb (shortened to the first
    /// and the last parts with a menu of the others when the row is narrow), the star, and at the
    /// right end the search and, in a narrow window, the command bar as icons.
    pub(crate) fn nav_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>, compact: bool) {
        let p = Self::palette_now(ui);
        let history = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => Some((doc.can_go(-1), doc.can_go(1))),
            _ => None,
        };
        let crumbs = self.nav_crumbs();
        let current = self.current_bookmark();
        let star = current.is_some();
        let marked = current.is_some_and(|b| self.marks.find(&b).is_some());
        let hint = self.search_hint_now();
        let search_open = self.active_search().is_some_and(|s| s.open);
        let tools = compact && self.has_tools();
        let tools_id = egui::Id::new("nav-tools-width");
        let tools_w = if tools {
            ui.data(|d| d.get_temp::<f32>(tools_id))
                .unwrap_or(TOOLS_WIDTH)
        } else {
            0.0
        };
        let search_w = if hint.is_empty() {
            0.0
        } else if search_open {
            SEARCH_OPEN_WIDTH
        } else {
            SEARCH_CLOSED_WIDTH
        };
        let clone_act = |a: &Act| match a {
            Act::Profile(ProfileAct::Select(s)) => {
                Some(Act::Profile(ProfileAct::Select(s.clone())))
            }
            Act::Profile(ProfileAct::SelectIn(s, g)) => {
                Some(Act::Profile(ProfileAct::SelectIn(s.clone(), g.clone())))
            }
            Act::Catalog(CatalogAct::Select(s)) => {
                Some(Act::Catalog(CatalogAct::Select(s.clone())))
            }
            _ => None,
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if let Some((back, forward)) = history {
                if style::icon_button(ui, icons::BACK, tr("Back (Alt+Left)"), back, None).clicked()
                {
                    acts.push(Act::Profile(ProfileAct::Back));
                }
                if style::icon_button(ui, icons::FORWARD, tr("Forward (Alt+Right)"), forward, None)
                    .clicked()
                {
                    acts.push(Act::Profile(ProfileAct::Forward));
                }
                ui.add_space(6.0);
            }
            let star_w = if star { STAR_WIDTH } else { 0.0 };
            let taken = tools_w + search_w + star_w + CRUMBS_MARGIN;
            let room = (ui.available_width() - taken).max(CRUMBS_MIN);
            let shown = fit_crumbs(ui, &crumbs, room);
            let hidden: Vec<usize> = (0..crumbs.len())
                .filter(|i| !shown.contains(&Some(*i)))
                .collect();
            for (k, entry) in shown.iter().enumerate() {
                if k > 0 {
                    ui.label(style::muted(ui, icons::CRUMB, 13.0));
                }
                match entry {
                    Some(i) => {
                        let (label, act) = &crumbs[*i];
                        let last = *i + 1 == crumbs.len();
                        let text = if last {
                            style::strong(ui, label.as_str(), 13.0)
                        } else {
                            RichText::new(label.as_str())
                        };
                        let clicked = if last && shown.len() == 1 {
                            ui.scope(|ui| {
                                ui.set_max_width(room);
                                ui.add(egui::Label::new(text).truncate().sense(Sense::click()))
                            })
                            .inner
                            .clicked()
                        } else {
                            style::flat_button(ui, text).clicked()
                        };
                        if clicked && let Some(a) = act.as_ref().and_then(clone_act) {
                            acts.push(a);
                        }
                    }
                    None => {
                        let more = style::flat_button(ui, RichText::new(icons::ELLIPSIS))
                            .on_hover_text(tr("The other parts of the path"));
                        egui::Popup::menu(&more).show(|ui| {
                            for i in &hidden {
                                let (label, act) = &crumbs[*i];
                                if ui.button(label.as_str()).clicked() {
                                    if let Some(a) = act.as_ref().and_then(clone_act) {
                                        acts.push(a);
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                }
            }
            if star {
                let (icon, label) = if marked {
                    (icons::STAR, tr("Remove the bookmark (Ctrl+B)"))
                } else {
                    (icons::STAR_EMPTY, tr("Bookmark this (Ctrl+B)"))
                };
                if style::icon_button(ui, icon, label, true, Some(p.accent)).clicked() {
                    acts.push(Act::Marks(MarksAct::Toggle));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let reserve = if tools { tools_w + 12.0 } else { 0.0 };
                self.search_field(ui, acts, hint, reserve);
                if tools {
                    ui.separator();
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(tools_w, 30.0), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    self.toolbar(&mut child, acts, true);
                    let used = child.min_rect().width().ceil();
                    if (used - tools_w).abs() > 0.5 {
                        ui.data_mut(|d| d.insert_temp(tools_id, used));
                        ui.ctx().request_repaint();
                    }
                }
            });
        });
    }

    /// The search of the active tab, at the right end of the navigation row.
    /// The search of the active tab; `reserve` is the width the row keeps for what comes after it.
    pub(crate) fn search_field(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Act>,
        hint: &'static str,
        reserve: f32,
    ) {
        if hint.is_empty() {
            return;
        }
        let history = self.search_history.clone();
        let Some(search) = self.active_search() else {
            return;
        };
        if !search.open {
            if style::icon_button(ui, icons::FIND, tr(hint), true, None).clicked() {
                search.open = true;
                search.focus = true;
            }
            return;
        }
        let had_text = !search.text.trim().is_empty();
        let mut remember = None;
        if !history.is_empty() {
            let more = style::icon_button(ui, icons::UNFOLDED, tr("Recent searches"), true, None);
            egui::Popup::menu(&more).show(|ui| {
                for h in &history {
                    if ui.button(h).clicked() {
                        search.text = h.clone();
                        ui.close();
                    }
                }
            });
        }
        let id = egui::Id::new("search-field");
        let width = (ui.available_width() - 40.0 - reserve).clamp(160.0, 560.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut search.text)
                .id(id)
                .hint_text(tr(hint))
                .margin(Margin::symmetric(8, 4))
                .desired_width(width),
        );
        if search.focus {
            ui.memory_mut(|m| m.request_focus(id));
            search.focus = false;
        }
        let close = style::icon_button(ui, icons::CLOSE, tr("Clear and close (Esc)"), true, None);
        let escaped = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
        if close.clicked() || escaped {
            search.text.clear();
            search.open = false;
        } else if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            remember = Some(search.text.trim().to_owned());
        }
        if had_text && search.text.trim().is_empty() {
            acts.push(Act::Nav(NavAct::SearchCleared));
        }
        if let Some(text) = remember
            && crate::remember_search(&mut self.search_history, &text)
        {
            self.settings.search_history = self.search_history.clone();
            self.settings_dirty = true;
        }
    }
}

/// The parts of the breadcrumb that fit a width: all of them, else the first, an ellipsis
/// (`None`) and the last ones, else the last alone.
fn fit_crumbs(ui: &egui::Ui, crumbs: &[(String, Option<Act>)], room: f32) -> Vec<Option<usize>> {
    let n = crumbs.len();
    let width = |i: usize| {
        let font = if i + 1 == n {
            style::strong_font(ui, 13.0)
        } else {
            style::font(ui, 13.0)
        };
        ui.painter()
            .layout_no_wrap(crumbs[i].0.clone(), font, egui::Color32::WHITE)
            .size()
            .x
            + 22.0
    };
    let total = |list: &[Option<usize>]| {
        list.iter().map(|x| x.map_or(26.0, width)).sum::<f32>()
            + 16.0 * list.len().saturating_sub(1) as f32
    };
    let mut list: Vec<Option<usize>> = (0..n).map(Some).collect();
    while total(&list) > room && list.len() > 2 {
        if list[1].is_some() {
            list[1] = None;
        } else if list.len() > 3 {
            list.remove(2);
        } else {
            break;
        }
    }
    if total(&list) > room && n > 0 {
        list = vec![Some(n - 1)];
    }
    list
}
