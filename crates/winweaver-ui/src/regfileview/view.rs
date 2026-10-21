//! The tab: the outline on the left, the selected key with its values and comments on the right.

use super::*;

/// The row of an entry in the outline.
pub(crate) fn outline_row(lines: &[Line], item: &Item, index: usize) -> tree::Row<Option<usize>> {
    let (title, right, icon, muted) = match item {
        Item::Key { line, lines: range } => {
            let (path, delete) = match &lines[*line] {
                Line::Key { path, delete } => (short_path(path), *delete),
                _ => (String::new(), false),
            };
            let values = range
                .clone()
                .filter(|&i| matches!(lines[i], Line::Value { .. }))
                .count();
            if delete {
                (path, String::new(), icons::DELETE, true)
            } else {
                (path, values.to_string(), icons::FOLDER, false)
            }
        }
        Item::Notes { lines: range } => {
            let first = range.clone().find_map(|i| match &lines[i] {
                Line::Comment(t) => Some(t.trim_start_matches(';').trim().to_owned()),
                Line::Other(t) => Some(t.trim().to_owned()),
                Line::Value { name, .. } => Some(value_title(name).to_owned()),
                Line::Blank | Line::Key { .. } => None,
            });
            let right = if range.len() > 1 {
                range.len().to_string()
            } else {
                String::new()
            };
            (first.unwrap_or_default(), right, ";", true)
        }
    };
    tree::Row {
        key: Some(index),
        depth: 0,
        title,
        right,
        check: tree::Check::None,
        open: None,
        muted,
        icon: Some(icon),
        switch: None,
    }
}

/// The outline: the whole file first, then the entries the search finds.
pub(crate) fn outline(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    items: &[Item],
    selected: Option<usize>,
) {
    let lines = &doc.file.lines;
    let words = doc.search.words();
    let keys = lines
        .iter()
        .filter(|l| matches!(l, Line::Key { .. }))
        .count();
    let mut rows = vec![tree::Row {
        key: None,
        depth: 0,
        title: tr("The whole file").to_owned(),
        right: keys.to_string(),
        check: tree::Check::None,
        open: None,
        muted: false,
        icon: Some(icons::NEW),
        switch: None,
    }];
    for (i, item) in items.iter().enumerate() {
        if !words.is_empty() {
            let text: Vec<String> = lines[item.lines()].iter().map(search_text).collect();
            let text = text.join("\n");
            if !matches_words(&text, &words) {
                continue;
            }
        }
        rows.push(outline_row(lines, item, i));
    }
    let shown_at = rows.iter().position(|r| r.key == selected);
    let scroll = tree::Scroll::wanted(false, doc.follow);
    let id = egui::Id::new(("regfile-outline", doc.path.display().to_string()));
    let shown = tree::show(ui, id, &rows, shown_at, scroll);
    if shown.scrolled || shown_at.is_none() {
        doc.follow = false;
    }
    if shown.moved {
        doc.follow = true;
    }
    for event in shown.events {
        if let tree::Event::Select(key) = event {
            doc.at = key.map(|i| items[i].anchor());
            doc.line = None;
            doc.error = None;
        }
    }
}

/// A rule as a link to the profile, with the links to the catalogs it is in.
pub(crate) fn rule_link(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = catalog.rule(id) else {
        return;
    };
    ui.horizontal_wrapped(|ui| {
        if ui.link(rtitle(texts, rule)).clicked() {
            acts.push(Act::Profile(ProfileAct::ShowInProfile(
                String::new(),
                id.to_owned(),
            )));
        }
        views::catalog_links(ui, imports, id, acts);
    });
}

/// The head of a key: its path, what the file does with it, the registry tab and the rules that
/// write it or its selected value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn key_head(
    ui: &mut egui::Ui,
    doc: &RegFileDoc,
    line: usize,
    values: usize,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    acts: &mut Vec<Act>,
) {
    let Some(Line::Key { path, delete }) = doc.file.lines.get(line) else {
        return;
    };
    let value = doc.selected_value();
    views::header(ui, |ui| {
        let title = egui::RichText::new(path.as_str())
            .font(style::mono(ui, 14.0))
            .strong();
        style::copy_label(ui, title, true);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if *delete {
                style::chip(ui, tr("Deleted with its subkeys"), true);
            } else {
                let count = if values == 1 {
                    tr("1 value").to_owned()
                } else {
                    trf("{n} values", &[("n", &values)])
                };
                style::chip(ui, &count, false);
            }
            if in_default_user(path) {
                style::chip(ui, tr("Default user profile"), false);
            }
            if ui.link(tr("Open in the registry")).clicked() {
                acts.push(Act::Registry(RegistryAct::Open(
                    path.clone(),
                    value.clone(),
                )));
            }
        });
        if in_default_user(path) {
            let text = tr(
                "The default user profile: load its hive here before the import, as the comment at the top of the file says.",
            );
            ui.label(style::muted(ui, text, 12.0));
        }
    });
    let rules = doc.rules_of(path, value.as_deref());
    views::block(ui, |ui| {
        match &value {
            Some(v) => {
                let name = value_title(v);
                style::section_title(ui, &trf("Rules that write {name}", &[("name", &name)]));
            }
            None => style::section_title(ui, tr("Rules that write it")),
        }
        ui.add_space(4.0);
        if rules.is_empty() {
            ui.weak(tr("No rule of the loaded catalogs writes it."));
        }
        for id in rules.iter().take(RULES_MAX) {
            rule_link(ui, catalog, texts, imports, id, acts);
        }
        if rules.len() > RULES_MAX {
            let more = rules.len() - RULES_MAX;
            ui.collapsing(trf("and {n} more", &[("n", &more)]), |ui| {
                for id in &rules[RULES_MAX..] {
                    rule_link(ui, catalog, texts, imports, id, acts);
                }
            });
        }
    });
    ui.add_space(10.0);
}

/// The head of the whole file: its name, its header, what it holds and how it is imported.
pub(crate) fn file_head(ui: &mut egui::Ui, doc: &RegFileDoc) {
    let lines = &doc.file.lines;
    let count = |f: &dyn Fn(&Line) -> bool| lines.iter().filter(|l| f(l)).count();
    let keys = count(&|l| matches!(l, Line::Key { delete: false, .. }));
    let gone_keys = count(&|l| matches!(l, Line::Key { delete: true, .. }));
    let values = count(&|l| matches!(l, Line::Value { data: Some(_), .. }));
    let gone_values = count(&|l| matches!(l, Line::Value { data: None, .. }));
    let comments = count(&|l| matches!(l, Line::Comment(_)));
    let other = count(&|l| matches!(l, Line::Other(_)));
    let default_user = lines
        .iter()
        .any(|l| matches!(l, Line::Key { path, .. } if in_default_user(path)));
    let facts = [
        (keys, trf("{n} keys", &[("n", &keys)])),
        (gone_keys, trf("{n} keys deleted", &[("n", &gone_keys)])),
        (values, trf("{n} values", &[("n", &values)])),
        (
            gone_values,
            trf("{n} values deleted", &[("n", &gone_values)]),
        ),
        (comments, trf("{n} comments", &[("n", &comments)])),
        (other, trf("{n} lines not read", &[("n", &other)])),
    ];
    views::header(ui, |ui| {
        ui.label(style::strong(ui, doc.title(), 18.0));
        let path = style::muted(ui, doc.path.display().to_string(), 12.0);
        style::copy_label(ui, path, true);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, &doc.file.header, true);
            for (n, text) in &facts {
                if *n > 0 {
                    style::chip(ui, text, false);
                }
            }
        });
    });
    views::block(ui, |ui| {
        style::section_title(ui, tr("Import"));
        ui.add_space(4.0);
        ui.label(tr(
            "Windows imports a registry file with regedit or reg import, as administrator for the keys of the computer. Try it on a test PC first.",
        ));
        if default_user {
            ui.label(tr(
                "Its keys of the default user profile need its hive loaded first, as the comment at the top of the file says.",
            ));
        }
        if other > 0 {
            let p = style::palette(ui);
            ui.colored_label(
                p.warn,
                tr("The lines the program does not read are kept as they are and saved again."),
            );
        }
    });
}

/// The lines of an entry as a list of name, type and data; the key itself is in the head.
pub(crate) fn lines_list(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    range: Range<usize>,
    key: Option<usize>,
    catalog: &Catalog,
    acts: &mut Vec<Act>,
) {
    let lines = &doc.file.lines;
    let key_path = key.and_then(|k| match &lines[k] {
        Line::Key { path, .. } => Some(path.clone()),
        _ => None,
    });
    let columns = [
        list::Column::new(tr("Name"), 220.0),
        list::Column::new(tr("Type"), 150.0),
        list::Column::new(tr("Data"), 400.0),
    ];
    let mut rows = Vec::new();
    for i in range {
        let (cells, button) = match &lines[i] {
            Line::Blank | Line::Key { .. } => continue,
            Line::Value { name, data } => {
                let (kind, shown) = match data {
                    Some(d) => (
                        list::Cell::new(d.type_name()).muted(),
                        list::Cell::new(short(&d.display())),
                    ),
                    None => (list::Cell::new(tr("deleted")).warn(), list::Cell::new("")),
                };
                let button = key_path
                    .as_ref()
                    .map(|_| (icons::REGISTRY, tr("Open in the registry").to_owned()));
                (
                    vec![list::Cell::new(value_title(name)), kind, shown],
                    button,
                )
            }
            Line::Comment(t) => {
                let button = rule_of_comment(t)
                    .filter(|id| catalog.rule(id).is_some())
                    .map(|_| (icons::SHOW, tr("Show in the profile").to_owned()));
                let text = t.strip_prefix(';').unwrap_or(t).trim();
                let cells = vec![
                    list::Cell::new(";").muted(),
                    list::Cell::new(tr("comment")).muted(),
                    list::Cell::new(text).muted(),
                ];
                (cells, button)
            }
            Line::Other(t) => {
                let cells = vec![
                    list::Cell::new(""),
                    list::Cell::new(tr("not read")).warn(),
                    list::Cell::new(t.as_str()),
                ];
                (cells, None)
            }
        };
        rows.push(list::Row {
            key: i,
            cells,
            button,
        });
    }
    let selected = doc.line.and_then(|l| rows.iter().position(|r| r.key == l));
    let scroll = if doc.follow_line {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    let id = egui::Id::new(("regfile-lines", doc.path.display().to_string()));
    let shown = list::show(ui, id, tr("Lines"), &columns, &rows, selected, scroll);
    if shown.scrolled || selected.is_none() {
        doc.follow_line = false;
    }
    if shown.moved {
        doc.follow_line = true;
    }
    for event in shown.events {
        match event {
            list::Event::Select(i) => {
                doc.line = Some(i);
                doc.error = None;
            }
            list::Event::Open(i) => {
                doc.line = Some(i);
                doc.start_edit(i);
            }
            list::Event::Button(i) => match &doc.file.lines[i] {
                Line::Value { name, .. } => {
                    if let Some(k) = &key_path {
                        acts.push(Act::Registry(RegistryAct::Open(
                            k.clone(),
                            Some(name.clone()),
                        )));
                    }
                }
                Line::Comment(t) => {
                    if let Some(id) = rule_of_comment(t) {
                        acts.push(Act::Profile(ProfileAct::ShowInProfile(
                            String::new(),
                            id.to_owned(),
                        )));
                    }
                }
                _ => {}
            },
            list::Event::Delete(i) => {
                doc.line = Some(i);
                acts.extend(doc.remove_question().map(Act::from));
            }
        }
    }
}

/// The tab: the outline on the left, the selected entry on the right, the editor over them.
pub fn regfile_view(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    acts: &mut Vec<Act>,
) {
    if let Some((_, path)) = doc.selected_key() {
        let value = doc.selected_value();
        regview::Writers::refresh(
            &mut doc.writers,
            catalog,
            &catalog_key(&path),
            value.as_deref(),
        );
    }
    let items = items(&doc.file.lines);
    let selected = doc.item(&items);
    let p = style::palette(ui);
    widgets::side_panel(ui, "regfile-outline", 380.0, |ui| {
        outline(ui, doc, &items, selected);
    });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg))
        .show(ui, |ui| {
            if let Some(e) = &doc.error {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.colored_label(p.warn, e);
                });
            }
            match selected.map(|i| &items[i]) {
                None => {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| file_head(ui, doc));
                }
                Some(Item::Key { line, lines }) => {
                    let values = lines
                        .clone()
                        .filter(|&i| matches!(doc.file.lines[i], Line::Value { .. }))
                        .count();
                    key_head(ui, doc, *line, values, catalog, texts, imports, acts);
                    lines_list(ui, doc, lines.clone(), Some(*line), catalog, acts);
                }
                Some(Item::Notes { lines }) => {
                    views::header(ui, |ui| {
                        ui.label(style::strong(ui, tr("Comments"), 16.0));
                        let n = lines.len();
                        let text = trf("{n} lines outside a key, kept in place", &[("n", &n)]);
                        ui.label(style::muted(ui, text, 12.0));
                    });
                    ui.add_space(10.0);
                    lines_list(ui, doc, lines.clone(), None, catalog, acts);
                }
            }
        });
    editor(ui.ctx(), doc);
}
