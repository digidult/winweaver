//! The bookmark tab (`docs/spec/01-ui.md` section 9): the bookmarks of the open bookmark file as a
//! list with columns, and on the right the selected one: its option with the source catalog when
//! it is not the built-in one, the comment of several lines, then the description of the option.

use std::path::Path;

use eframe::egui::{self, Margin};
use winweaver_core::admx::CatalogFile;
use winweaver_core::bookmarks::{Bookmark, Bookmarks};
use winweaver_core::catalog::{Catalog, Rule};
use winweaver_core::i18n::CatalogTexts;

use crate::ShellAct;
use crate::i18n::{tr, trf};
use crate::views::{self, gsummary, gtitle, rtitle};
use crate::{
    Act, CatalogAct, Form, MarksAct, ProfileAct, Question, Settings, icons, list, matches_words,
    style, tree,
};

/// The question before the removal of a bookmark, naming its option and its comment.
pub fn remove_question(
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    marks: &Bookmarks,
    i: usize,
) -> Option<Question> {
    let b = marks.items.get(i)?;
    let t = target(catalog, texts, imports, b);
    let text = if b.comment.trim().is_empty() {
        trf("Remove the bookmark of {title}?", &[("title", &t.title)])
    } else {
        trf(
            "The bookmark of {title} has a comment. Remove it with the comment?",
            &[("title", &t.title)],
        )
    };
    Some(Question::new(
        tr("Remove the bookmark"),
        text,
        tr("Remove"),
        MarksAct::Remove(i),
    ))
}

/// The bookmark tab.
#[derive(Default)]
pub struct MarksDoc {
    pub search: crate::Search,
    /// The index of the selected bookmark in the file.
    pub sel: Option<usize>,
    /// The selection moved with the keys: it comes into view at the next frame.
    pub follow: bool,
    /// The search was cleared: the selection comes to the middle of the list.
    pub reveal: bool,
}

/// What a bookmark points at, as the tab shows it.
pub struct Target<'a> {
    pub title: String,
    pub group: String,
    /// The import the option comes from, when it is not in the built-in catalog.
    pub source: Option<String>,
    pub rule: Option<&'a Rule>,
    /// The summary of a group.
    pub summary: String,
    /// Not found in the catalog of the program.
    pub missing: bool,
}

/// The option of a bookmark in the catalog, with the import it comes from.
pub fn target<'a>(
    catalog: &'a Catalog,
    texts: Option<&'a CatalogTexts>,
    imports: &[CatalogFile],
    b: &Bookmark,
) -> Target<'a> {
    let mut t = Target {
        title: if b.title.is_empty() {
            b.item.clone()
        } else {
            b.title.clone()
        },
        group: String::new(),
        source: None,
        rule: None,
        summary: String::new(),
        missing: false,
    };
    if b.is_registry() {
        t.title = if b.item.is_empty() {
            b.path.rsplit('\\').next().unwrap_or(&b.path).to_owned()
        } else {
            b.item.clone()
        };
        t.group = b.path.clone();
        return t;
    }
    if !b.is_option() {
        t.group = b.document.clone();
        return t;
    }
    if let Some(rule) = catalog.rule(&b.item) {
        t.title = rtitle(texts, rule).to_owned();
        t.group = catalog
            .group(&rule.group)
            .map_or_else(|| rule.group.clone(), |g| gtitle(texts, g).to_owned());
        t.source = imports
            .iter()
            .find(|f| f.rules.iter().any(|r| r.id == b.item))
            .map(|f| f.name.clone());
        t.rule = Some(rule);
    } else if let Some(group) = catalog.group(&b.item) {
        t.title = gtitle(texts, group).to_owned();
        t.group = group
            .parent
            .as_deref()
            .and_then(|p| catalog.group(p))
            .map(|g| gtitle(texts, g).to_owned())
            .unwrap_or_default();
        t.source = imports
            .iter()
            .find(|f| f.groups.iter().any(|g| g.id == b.item))
            .map(|f| f.name.clone());
        t.summary = gsummary(texts, group).to_owned();
    } else if let Some(form) = Form::ALL.iter().find(|f| f.id() == b.item) {
        t.title = tr(form.title()).to_owned();
        t.group = tr("Installation").to_owned();
    } else {
        t.missing = true;
    }
    t
}

/// The file name of the profile of a bookmark.
fn profile_name(b: &Bookmark) -> String {
    if b.path.is_empty() {
        return String::new();
    }
    Path::new(&b.path)
        .file_name()
        .map_or_else(|| b.path.clone(), |n| n.to_string_lossy().into_owned())
}

/// The indexes of the bookmarks the search lets through.
fn shown(
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    marks: &Bookmarks,
    search: &crate::Search,
) -> Vec<usize> {
    let words = search.words();
    let index = (!words.is_empty()).then(|| catalog.search_index(texts));
    (0..marks.items.len())
        .filter(|&i| {
            if words.is_empty() {
                return true;
            }
            let b = &marks.items[i];
            let t = target(catalog, texts, imports, b);
            let rule = index.as_ref().map_or("", |i| i.text(&b.item));
            let hay = format!(
                "{} {} {} {} {}\n{rule}\n{}",
                t.title,
                t.group,
                b.item,
                b.path,
                b.comment,
                winweaver_core::registry::key_forms(&b.path)
            );
            matches_words(&hay, &words)
        })
        .collect()
}

/// The list of the bookmarks in the middle of the tab.
#[allow(clippy::too_many_arguments)]
pub fn marks_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    marks: &Bookmarks,
    path: &Path,
    search: &crate::Search,
    tab: &MarksDoc,
    acts: &mut Vec<Act>,
) {
    views::header(ui, |ui| {
        ui.label(style::strong(ui, tr("Bookmarks"), 18.0));
        ui.label(style::muted(
            ui,
            trf(
                "{path} ({n} bookmarks)",
                &[("path", &path.display()), ("n", &marks.items.len())],
            ),
            12.0,
        ));
    });
    if marks.items.is_empty() {
        egui::Frame::new()
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                ui.label(style::muted(
                    ui,
                    tr("No bookmarks yet. The star of the navigation row marks the selected option."),
                    13.0,
                ));
            });
        return;
    }
    let indexes = shown(catalog, texts, imports, marks, search);
    let columns = [
        list::Column::new(tr("Option"), 300.0),
        list::Column::new(tr("Group"), 180.0),
        list::Column::new(tr("Profile"), 180.0),
        list::Column::new(tr("Comment"), 200.0),
    ];
    let rows: Vec<list::Row<usize>> = indexes
        .iter()
        .map(|&i| {
            let b = &marks.items[i];
            let t = target(catalog, texts, imports, b);
            let title = if t.missing {
                list::Cell::new(trf("{id} (not in this catalog)", &[("id", &t.title)])).muted()
            } else {
                list::Cell::new(t.title)
            };
            list::Row {
                key: i,
                cells: vec![
                    title,
                    list::Cell::new(t.group).muted(),
                    list::Cell::new(profile_name(b)).muted(),
                    list::Cell::new(b.comment.clone()).muted(),
                ],
                button: Some((icons::GO_TO, open_hint(b))),
            }
        })
        .collect();
    let selected = tab.sel.and_then(|s| indexes.iter().position(|&i| i == s));
    let scroll = tree::Scroll::wanted(tab.reveal, tab.follow);
    let out = list::show(
        ui,
        egui::Id::new("marks-list"),
        tr("Bookmarks"),
        &columns,
        &rows,
        selected,
        scroll,
    );
    for event in out.events {
        match event {
            list::Event::Select(i) => acts.push(Act::Marks(MarksAct::Select(i))),
            list::Event::Open(i) | list::Event::Button(i) => {
                acts.push(Act::Marks(MarksAct::GoTo(i)))
            }
            list::Event::Delete(i) => {
                acts.extend(remove_question(catalog, texts, imports, marks, i).map(Act::from));
            }
        }
    }
    if out.moved {
        acts.push(Act::Shell(ShellAct::Follow));
    } else if out.scrolled {
        acts.push(Act::Shell(ShellAct::Revealed));
    }
}

fn open_hint(b: &Bookmark) -> String {
    if b.is_registry() {
        tr("Open in the registry").to_owned()
    } else if b.path.is_empty() {
        tr("Show the option in the profile in front").to_owned()
    } else {
        trf("Open {name} at this option", &[("name", &profile_name(b))])
    }
}

/// The detail panel on the right: the selected bookmark, its comment and its description.
#[allow(clippy::too_many_arguments)]
pub fn marks_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    marks: &Bookmarks,
    tab: &MarksDoc,
    settings: &Settings,
    acts: &mut Vec<Act>,
) {
    let Some((i, b)) = tab.sel.and_then(|i| marks.items.get(i).map(|b| (i, b))) else {
        egui::Frame::new()
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                ui.label(style::muted(
                    ui,
                    tr("Select a bookmark to see it here."),
                    13.0,
                ));
            });
        return;
    };
    let t = target(catalog, texts, imports, b);
    egui::ScrollArea::vertical()
        .id_salt("marks-detail")
        .auto_shrink(false)
        .show(ui, |ui| {
            views::header(ui, |ui| {
                ui.label(style::strong(ui, &t.title, 18.0));
                ui.horizontal(|ui| {
                    let p = style::palette(ui);
                    if style::icon_button(
                        ui,
                        icons::STAR,
                        tr("Remove the bookmark"),
                        true,
                        Some(p.accent),
                    )
                    .clicked()
                    {
                        acts.extend(
                            remove_question(catalog, texts, imports, marks, i).map(Act::from),
                        );
                    }
                    if !b.item.is_empty() {
                        views::id_line(ui, &b.item);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    match t.rule {
                        // The group of an option leads to it in the profile.
                        Some(rule) if !t.group.is_empty() => {
                            if ui.link(t.group.as_str()).clicked() {
                                acts.push(Act::Profile(ProfileAct::ShowGroupInProfile(
                                    rule.group.clone(),
                                )));
                            }
                        }
                        _ if !t.group.is_empty() => {
                            style::chip(ui, &t.group, false);
                        }
                        _ => {}
                    }
                    if let Some(source) = &t.source {
                        // The import of the option, a link to its catalog tab at it.
                        let text = trf("From {name}", &[("name", source)]);
                        let file = imports.iter().find(|f| &f.name == source);
                        match (file, t.rule) {
                            (Some(f), Some(rule)) => {
                                if ui.link(text).clicked() {
                                    acts.push(Act::Catalog(CatalogAct::OpenAt(
                                        f.id.clone(),
                                        rule.id.clone(),
                                    )));
                                }
                            }
                            _ => {
                                style::chip(ui, &text, true);
                            }
                        }
                    }
                    if t.missing {
                        style::chip(ui, tr("not in this catalog"), false);
                    }
                });
            });
            views::block(ui, |ui| {
                let title = if b.is_registry() {
                    tr("Registry")
                } else {
                    tr("Profile")
                };
                style::section_title(ui, title);
                ui.add_space(4.0);
                if b.is_registry() {
                    style::selectable_text(ui, ("mark-key", i), &b.path);
                } else if b.path.is_empty() {
                    ui.label(style::muted(
                        ui,
                        tr("No profile file: the option is shown in the profile in front."),
                        12.0,
                    ));
                } else {
                    style::selectable_text(ui, ("mark-path", i), &b.path);
                }
                ui.add_space(4.0);
                if ui.button(open_hint(b)).clicked() {
                    acts.push(Act::Marks(MarksAct::GoTo(i)));
                }
            });
            if b.is_registry() {
                let value = (!b.item.is_empty()).then_some(b.item.as_str());
                let rules = winweaver_core::registry::writers(catalog, &b.path, value);
                views::block(ui, |ui| {
                    style::section_title(ui, tr("Rules that write it"));
                    ui.add_space(4.0);
                    if rules.is_empty() {
                        ui.weak(tr("No rule of the loaded catalogs writes it."));
                    }
                    for rule in rules {
                        ui.horizontal_wrapped(|ui| {
                            if ui.link(rtitle(texts, rule)).clicked() {
                                acts.push(Act::Profile(ProfileAct::ShowInProfile(
                                    String::new(),
                                    rule.id.clone(),
                                )));
                            }
                            views::catalog_links(ui, imports, &rule.id, acts);
                        });
                    }
                });
            }
            views::block(ui, |ui| {
                style::section_title(ui, tr("Comment"));
                ui.add_space(4.0);
                let mut comment = b.comment.clone();
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut comment)
                        .id(egui::Id::new(("mark-comment", i)))
                        .hint_text(tr("A comment on this bookmark"))
                        .desired_rows(1)
                        .desired_width(f32::INFINITY),
                );
                if edit.changed() {
                    acts.push(Act::Marks(MarksAct::SetComment(i, comment)));
                }
            });
            if let Some(rule) = t.rule
                && imports
                    .iter()
                    .any(|f| f.rules.iter().any(|r| r.id == rule.id))
            {
                views::block(ui, |ui| {
                    style::section_title(ui, tr("In the catalogs"));
                    ui.add_space(4.0);
                    views::catalog_links(ui, imports, &rule.id, acts);
                });
            }
            if let Some(rule) = t.rule {
                let link = views::RuleLink::Elsewhere;
                views::rule_description(ui, catalog, texts, rule, settings, link, acts);
            } else if !t.summary.is_empty() {
                views::block(ui, |ui| {
                    style::section_title(ui, tr("Description"));
                    ui.add_space(4.0);
                    style::selectable_text(ui, ("mark-summary", i), &t.summary);
                });
            }
            ui.add_space(12.0);
        });
}
