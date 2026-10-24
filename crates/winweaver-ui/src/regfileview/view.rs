//! The tab: the outline on the left, the selected key with its values and comments on the right.

use super::*;

/// A row of the outline: the whole file, an entry of the file, or a part of a path no key of the
/// file names (a folder of the tree).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    File,
    Item(usize),
    Folder(String),
}

/// The title, the count, the icon and whether an entry is shown muted.
fn row_parts(lines: &[Line], item: &Item) -> (String, String, &'static str, bool) {
    match item {
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
    }
}

/// The row of an entry in the list of the outline.
pub(crate) fn outline_row(lines: &[Line], item: &Item, index: usize) -> tree::Row<Node> {
    let (title, right, icon, muted) = row_parts(lines, item);
    tree::Row {
        key: Node::Item(index),
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

/// The path of the key of an entry, in lower case, for the folded keys of the tree.
fn item_path(lines: &[Line], item: &Item) -> Option<String> {
    match item {
        Item::Key { line, .. } => match &lines[*line] {
            Line::Key { path, .. } => Some(path.to_lowercase()),
            _ => None,
        },
        Item::Notes { .. } => None,
    }
}

/// The keys of the file as a tree of their paths, as regedit shows them: a part of a path no key
/// of the file names is a folder; the comments outside the keys follow at the end.
pub(crate) fn tree_rows(
    lines: &[Line],
    items: &[Item],
    closed: &HashSet<String>,
) -> Vec<tree::Row<Node>> {
    struct Part {
        name: String,
        path: String,
        item: Option<usize>,
        children: Vec<usize>,
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut roots: Vec<usize> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        let Item::Key { line, .. } = item else {
            continue;
        };
        let Line::Key { path, .. } = &lines[*line] else {
            continue;
        };
        let mut parent: Option<usize> = None;
        let mut current = String::new();
        for name in path.split('\\').filter(|p| !p.is_empty()) {
            if !current.is_empty() {
                current.push('\\');
            }
            current.push_str(name);
            let known = index.get(&current.to_lowercase()).copied();
            let n = match known {
                Some(n) => n,
                None => {
                    parts.push(Part {
                        name: name.to_owned(),
                        path: current.clone(),
                        item: None,
                        children: Vec::new(),
                    });
                    let n = parts.len() - 1;
                    index.insert(current.to_lowercase(), n);
                    match parent {
                        Some(p) => parts[p].children.push(n),
                        None => roots.push(n),
                    }
                    n
                }
            };
            parent = Some(n);
        }
        if let Some(n) = parent
            && parts[n].item.is_none()
        {
            parts[n].item = Some(i);
        }
    }
    let mut rows = Vec::new();
    let mut stack: Vec<(usize, usize)> = roots.iter().rev().map(|&n| (n, 0)).collect();
    while let Some((n, depth)) = stack.pop() {
        let part = &parts[n];
        let open = !closed.contains(&part.path.to_lowercase());
        let (right, icon, muted) = match part.item {
            Some(i) => {
                let (_, right, icon, muted) = row_parts(lines, &items[i]);
                (right, icon, muted)
            }
            None => (String::new(), icons::FOLDER, true),
        };
        rows.push(tree::Row {
            key: part
                .item
                .map_or(Node::Folder(part.path.clone()), Node::Item),
            depth,
            title: part.name.clone(),
            right,
            check: tree::Check::None,
            open: (!part.children.is_empty()).then_some(open),
            muted,
            icon: Some(icon),
            switch: None,
        });
        if open {
            for &c in part.children.iter().rev() {
                stack.push((c, depth + 1));
            }
        }
    }
    for (i, item) in items.iter().enumerate() {
        if matches!(item, Item::Notes { .. }) {
            rows.push(outline_row(lines, item, i));
        }
    }
    rows
}

/// The outline: the whole file first, then its keys as a tree (`as_tree`) or the entries as a
/// list in the order of the file; a search lists the entries it finds.
pub(crate) fn outline(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    items: &[Item],
    selected: Option<usize>,
    as_tree: bool,
) {
    let lines = &doc.file.lines;
    let words = doc.search.words();
    let keys = lines
        .iter()
        .filter(|l| matches!(l, Line::Key { .. }))
        .count();
    let mut rows = vec![tree::Row {
        key: Node::File,
        depth: 0,
        title: tr("The whole file").to_owned(),
        right: keys.to_string(),
        check: tree::Check::None,
        open: None,
        muted: false,
        icon: Some(icons::NEW),
        switch: None,
    }];
    if as_tree && words.is_empty() {
        rows.extend(tree_rows(lines, items, &doc.closed));
    } else {
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
    }
    let wanted = selected.map_or(Node::File, Node::Item);
    let shown_at = rows.iter().position(|r| r.key == wanted);
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
        match event {
            tree::Event::Select(Node::File) => {
                doc.at = None;
                doc.line = None;
                doc.error = None;
            }
            tree::Event::Select(Node::Item(i)) => {
                doc.at = Some(items[i].anchor());
                doc.line = None;
                doc.error = None;
            }
            tree::Event::Expand(node, open) => {
                let path = match node {
                    Node::Folder(path) => Some(path.to_lowercase()),
                    Node::Item(i) => item_path(&doc.file.lines, &items[i]),
                    Node::File => None,
                };
                if let Some(path) = path {
                    if open {
                        doc.closed.remove(&path);
                    } else {
                        doc.closed.insert(path);
                    }
                }
            }
            _ => {}
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
    writers_open: bool,
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
            crate::polview::link(ui, path, value.as_deref(), acts);
        });
        if in_default_user(path) {
            let text = tr(
                "The default user profile: load its hive here before the import, as the comment at the top of the file says.",
            );
            ui.label(style::muted(ui, text, 12.0));
        }
    });
    let rules = doc.rules_of(path, value.as_deref());
    // A panel that folds, at most a third of the window high and scrolled inside, so the lines
    // of the key stay in view however many rules write it.
    let most = (ui.ctx().content_rect().height() / 3.0).max(120.0);
    views::block(ui, |ui| {
        let title = match &value {
            Some(v) => {
                let name = value_title(v);
                trf(
                    "Rules that write {name} ({n})",
                    &[("name", &name), ("n", &rules.len())],
                )
            }
            None => trf("Rules that write it ({n})", &[("n", &rules.len())]),
        };
        egui::CollapsingHeader::new(style::strong(ui, title, 13.0))
            .id_salt(("regfile-writers", writers_open))
            .default_open(writers_open)
            .show(ui, |ui| {
                if rules.is_empty() {
                    ui.weak(tr("No rule of the loaded catalogs writes it."));
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt("regfile-writers-scroll")
                    .max_height(most)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for id in &rules {
                            rule_link(ui, catalog, texts, imports, id, acts);
                        }
                    });
            });
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
/// What the settings say of the tab of a registry file.
pub struct Options {
    /// The outline is a tree of the keys.
    pub tree: bool,
    /// The rules that write a key are open at first.
    pub writers_open: bool,
}

pub fn regfile_view(
    ui: &mut egui::Ui,
    doc: &mut RegFileDoc,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    options: &Options,
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
        outline(ui, doc, &items, selected, options.tree);
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
                    key_head(
                        ui,
                        doc,
                        *line,
                        values,
                        catalog,
                        texts,
                        imports,
                        options.writers_open,
                        acts,
                    );
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
