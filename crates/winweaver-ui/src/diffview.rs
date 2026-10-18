//! The diff tab: the differences between two profiles (`docs/spec/01-ui.md` section 7) as a list
//! with columns, and on the right the item selected in it: its two values, a comment kept in the
//! diff file above the description of its rule, and the way to it in the profiles of both sides.

use std::path::PathBuf;

use eframe::egui::{self, Margin, RichText};
use serde_json::Value;
use winweaver_core::catalog::{Catalog, Rule};
use winweaver_core::diff::{self, Diff};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::profile::Choice;

use crate::i18n::{tr, trf};
use crate::views::{self, gtitle, rtitle};
use crate::{Act, Settings, list, style, tree};

pub struct DiffDoc {
    pub diff: Diff,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub search: crate::Search,
    /// The key of the selected item ([`Diff::keys`]).
    pub sel: Option<String>,
    /// The selection moved with the keys: it comes into view at the next frame.
    pub follow: bool,
    /// The search was cleared: the selection comes to the middle of the list.
    pub reveal: bool,
}

impl DiffDoc {
    pub fn new(diff: Diff, path: Option<PathBuf>) -> Self {
        Self {
            diff,
            path,
            dirty: false,
            search: crate::Search::default(),
            sel: None,
            follow: false,
            reveal: false,
        }
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => format!(
                "{} vs {}.{}",
                self.diff.base.name,
                self.diff.other.name,
                diff::EXTENSION
            ),
        }
    }
}

/// The state of a rule in one profile as lines: on or off, then the parameters.
fn choice_lines(choice: &Option<Choice>) -> Vec<String> {
    match choice {
        None => vec![tr("off").to_owned()],
        Some(c) => {
            let mut lines = vec![if c.on { tr("on") } else { tr("off") }.to_owned()];
            for (name, value) in &c.params {
                lines.push(format!("{name} = {}", value_text(value)));
            }
            lines
        }
    }
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|v| match v {
                Value::Object(o) => o
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_owned(),
                other => value_text(other),
            })
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

/// An item of a diff as the list and the detail panel show it.
pub struct Item<'a> {
    pub key: String,
    pub title: String,
    /// The group of a rule, or the form of a field.
    pub group: String,
    pub base: Vec<String>,
    pub other: Vec<String>,
    pub rule: Option<&'a Rule>,
}

/// The items of a diff: the fields of the forms, then the rules in the order of the catalog,
/// then the rules this catalog does not have.
pub fn items<'a>(catalog: &'a Catalog, texts: Option<&CatalogTexts>, d: &Diff) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    for (k, f) in &d.install {
        items.push(Item {
            key: format!("install:{k}"),
            title: k.clone(),
            group: tr("Installation").to_owned(),
            base: vec![value_text(&f.base)],
            other: vec![value_text(&f.other)],
            rule: None,
        });
    }
    for (k, f) in &d.languages {
        items.push(Item {
            key: format!("languages:{k}"),
            title: k.clone(),
            group: tr("Languages").to_owned(),
            base: vec![value_text(&f.base)],
            other: vec![value_text(&f.other)],
            rule: None,
        });
    }
    if let Some(f) = &d.accounts {
        items.push(Item {
            key: "accounts".to_owned(),
            title: tr("Accounts").to_owned(),
            group: tr("Accounts").to_owned(),
            base: vec![value_text(&f.base)],
            other: vec![value_text(&f.other)],
            rule: None,
        });
    }
    for rule in catalog.rules() {
        let Some(r) = d.rules.get(&rule.id) else {
            continue;
        };
        items.push(Item {
            key: format!("rule:{}", rule.id),
            title: rtitle(texts, rule).to_owned(),
            group: catalog
                .group(&rule.group)
                .map_or_else(|| rule.group.clone(), |g| gtitle(texts, g).to_owned()),
            base: choice_lines(&r.base),
            other: choice_lines(&r.other),
            rule: Some(rule),
        });
    }
    for (id, r) in &d.rules {
        if catalog.rule(id).is_some() {
            continue;
        }
        items.push(Item {
            key: format!("rule:{id}"),
            title: trf("{id} (not in this catalog)", &[("id", id)]),
            group: String::new(),
            base: choice_lines(&r.base),
            other: choice_lines(&r.other),
            rule: None,
        });
    }
    items
}

/// The items the search of the tab lets through.
fn shown<'a>(catalog: &'a Catalog, texts: Option<&CatalogTexts>, doc: &DiffDoc) -> Vec<Item<'a>> {
    let words = doc.search.words();
    let mut items = items(catalog, texts, &doc.diff);
    if !words.is_empty() {
        items.retain(|item| {
            let hay = format!(
                "{} {} {} {} {} {}",
                item.key,
                item.title,
                item.group,
                item.base.join(" "),
                item.other.join(" "),
                doc.diff.comments.get(&item.key).map_or("", String::as_str)
            )
            .to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        });
    }
    items
}

fn side_path(path: &str) -> &str {
    if path.is_empty() {
        tr("not saved")
    } else {
        path
    }
}

/// The list of the differences in the middle of the tab.
pub fn diff_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &DiffDoc,
    acts: &mut Vec<Act>,
) {
    let d = &doc.diff;
    views::header(ui, |ui| {
        ui.label(style::strong(
            ui,
            format!("{} vs {}", d.base.name, d.other.name),
            18.0,
        ));
        ui.label(style::muted(
            ui,
            trf(
                "{n} differences; left: {left}, right: {right}",
                &[
                    ("n", &d.count()),
                    ("left", &side_path(&d.base.path)),
                    ("right", &side_path(&d.other.path)),
                ],
            ),
            12.0,
        ));
    });
    let items = shown(catalog, texts, doc);
    let columns = [
        list::Column::new(tr("Item"), 280.0),
        list::Column::new(tr("Group"), 170.0),
        list::Column::new(&d.base.name, 170.0),
        list::Column::new(&d.other.name, 170.0),
        list::Column::new(tr("Comment"), 160.0),
    ];
    let rows: Vec<list::Row<String>> = items
        .iter()
        .map(|item| list::Row {
            key: item.key.clone(),
            cells: vec![
                if item.rule.is_some() || !item.key.starts_with("rule:") {
                    list::Cell::new(&item.title)
                } else {
                    list::Cell::new(&item.title).muted()
                },
                list::Cell::new(&item.group).muted(),
                list::Cell::new(item.base.join(", ")).chip(false),
                list::Cell::new(item.other.join(", ")).chip(true),
                list::Cell::new(
                    doc.diff
                        .comments
                        .get(&item.key)
                        .cloned()
                        .unwrap_or_default(),
                )
                .muted(),
            ],
            button: None,
        })
        .collect();
    let selected = doc
        .sel
        .as_ref()
        .and_then(|k| items.iter().position(|i| &i.key == k));
    let scroll = if doc.reveal {
        tree::Scroll::Center
    } else if doc.follow {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    let out = list::show(
        ui,
        egui::Id::new((
            "diff-list",
            doc.diff.base.name.as_str(),
            doc.diff.other.name.as_str(),
        )),
        tr("Differences"),
        &columns,
        &rows,
        selected,
        scroll,
    );
    for event in out.events {
        match event {
            list::Event::Select(key) => acts.push(Act::DiffSelect(key)),
            list::Event::Open(key) | list::Event::Button(key) => {
                acts.push(Act::DiffGoTo(key, false))
            }
            list::Event::Delete(_) => {}
        }
    }
    if out.moved {
        acts.push(Act::Follow);
    } else if out.scrolled {
        acts.push(Act::Revealed);
    }
}

/// A side of the difference: a card with the name of its profile and its value, tinted red for
/// the base and green for the other side.
fn side_card(ui: &mut egui::Ui, name: &str, lines: &[String], new: bool) {
    let p = style::palette(ui);
    let tint = if new { p.ok } else { p.warn };
    egui::Frame::new()
        .fill(tint.gamma_multiply(if p.dark { 0.18 } else { 0.08 }))
        .stroke(egui::Stroke::new(1.0, tint.gamma_multiply(0.5)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            style::section_title(ui, name);
            for line in lines {
                ui.label(line.as_str());
            }
        });
}

/// The detail panel on the right: the selected item, its comment and the description of its rule.
/// `marked` says whether its rule has a bookmark.
pub fn diff_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &DiffDoc,
    settings: &Settings,
    marked: bool,
    acts: &mut Vec<Act>,
) {
    let d = &doc.diff;
    let Some(item) = doc
        .sel
        .as_ref()
        .and_then(|k| items(catalog, texts, d).into_iter().find(|i| &i.key == k))
    else {
        egui::Frame::new()
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                ui.label(style::muted(
                    ui,
                    tr("Select a difference to see it here."),
                    13.0,
                ));
            });
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt("diff-detail")
        .auto_shrink(false)
        .show(ui, |ui| {
            views::header(ui, |ui| {
                ui.label(style::strong(ui, &item.title, 18.0));
                if let Some(id) = item.key.strip_prefix("rule:") {
                    ui.horizontal(|ui| {
                        let p = style::palette(ui);
                        let (icon, hint) = if marked {
                            ("\u{2605}", tr("Remove the bookmark (Ctrl+B)"))
                        } else {
                            ("\u{2606}", tr("Bookmark this (Ctrl+B)"))
                        };
                        if style::icon_button(ui, icon, hint, true, Some(p.accent)).clicked() {
                            acts.push(Act::ToggleBookmark);
                        }
                        views::id_line(ui, id);
                    });
                }
                if !item.group.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        style::chip(ui, &item.group, false);
                    });
                }
            });
            views::block(ui, |ui| {
                style::section_title(ui, tr("Difference"));
                ui.add_space(4.0);
                ui.columns(2, |cols| {
                    side_card(&mut cols[0], &d.base.name, &item.base, false);
                    side_card(&mut cols[1], &d.other.name, &item.other, true);
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(trf("Show in {name}", &[("name", &d.base.name)]))
                        .on_hover_text(tr("Selects it in the profile tab, opening the profile when needed"))
                        .clicked()
                    {
                        acts.push(Act::DiffGoTo(item.key.clone(), false));
                    }
                    if ui
                        .button(trf("Show in {name}", &[("name", &d.other.name)]))
                        .on_hover_text(tr("Selects it in the profile tab, opening the profile when needed"))
                        .clicked()
                    {
                        acts.push(Act::DiffGoTo(item.key.clone(), true));
                    }
                    if ui
                        .button(tr("Apply this difference"))
                        .on_hover_text(trf(
                            "Sets it in {base} as in {other}",
                            &[("base", &d.base.name), ("other", &d.other.name)],
                        ))
                        .clicked()
                    {
                        acts.push(Act::DiffApplyItem(item.key.clone()));
                    }
                });
            });
            views::block(ui, |ui| {
                style::section_title(ui, tr("Comment"));
                ui.add_space(4.0);
                let mut comment = d.comments.get(&item.key).cloned().unwrap_or_default();
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut comment)
                        .id(egui::Id::new(("diff-comment", item.key.as_str())))
                        .hint_text(tr("A comment on this difference, saved in the diff file"))
                        .desired_rows(1)
                        .desired_width(f32::INFINITY),
                );
                if edit.changed() {
                    acts.push(Act::DiffComment(item.key.clone(), comment));
                }
            });
            match item.rule {
                Some(rule) => {
                    views::rule_description(ui, catalog, texts, rule, settings, false, acts)
                }
                None if item.key.starts_with("rule:") => {
                    views::block(ui, |ui| {
                        ui.label(RichText::new(tr(
                            "This rule is not in the catalog of this program, so it has no description here.",
                        )));
                    });
                }
                None => {}
            }
            ui.add_space(12.0);
        });
}
