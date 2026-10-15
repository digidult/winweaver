//! The tab of a catalog of policy templates (`docs/spec/01-ui.md` section 8): an import, or a
//! catalog file opened like any document without importing it. Its tree has the rows of a profile
//! but no check boxes; the detail shows a policy, its parameters and registry values. The search
//! of the tab filters the policies.

use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui::{self, RichText};
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{Group, Rule};

use crate::i18n::{tr, trf};
use crate::{Act, Search, style, tree};

/// An open tab of a catalog.
pub struct CatalogDoc {
    /// The id of the import, or of the file.
    pub id: String,
    /// The name when the tab was opened.
    pub name: String,
    /// A catalog file opened without importing it, and its path.
    pub file: Option<Box<CatalogFile>>,
    pub path: Option<PathBuf>,
    /// The selected rule.
    pub sel: Option<String>,
    /// The selected group, when no rule is.
    pub group: Option<String>,
    pub expanded: HashSet<String>,
    pub search: Search,
    /// The selected row is scrolled to the middle at the next frame.
    pub reveal: bool,
    /// The selected row follows into view at the next frame.
    pub follow: bool,
}

impl CatalogDoc {
    /// The tab of an import.
    pub fn new(file: &CatalogFile) -> Self {
        Self {
            id: file.id.clone(),
            name: file.name.clone(),
            file: None,
            path: None,
            sel: None,
            group: None,
            expanded: roots(file).map(|g| g.id.clone()).collect(),
            search: Search::default(),
            reveal: false,
            follow: false,
        }
    }

    /// The tab of a catalog file that is not imported.
    pub fn from_file(file: CatalogFile, path: PathBuf) -> Self {
        let mut doc = Self::new(&file);
        doc.file = Some(Box::new(file));
        doc.path = Some(path);
        doc
    }

    /// The catalog the tab shows: its own file, or the import of the same id.
    pub fn catalog<'a>(&'a self, imports: &'a [CatalogFile]) -> Option<&'a CatalogFile> {
        match &self.file {
            Some(f) => Some(&**f),
            None => imports.iter().find(|f| f.id == self.id),
        }
    }
}

/// The groups of a file whose parent is outside it.
fn roots(file: &CatalogFile) -> impl Iterator<Item = &Group> {
    file.groups.iter().filter(|g| {
        !file
            .groups
            .iter()
            .any(|p| g.parent.as_deref() == Some(p.id.as_str()))
    })
}

pub(crate) fn group_title<'a>(file: &'a CatalogFile, lang: &str, group: &'a Group) -> &'a str {
    file.texts
        .get(lang)
        .and_then(|t| t.groups.get(&group.id))
        .map_or(group.title.as_str(), String::as_str)
}

pub(crate) fn rule_texts<'a>(
    file: &'a CatalogFile,
    lang: &str,
    rule: &'a Rule,
) -> (&'a str, &'a str) {
    match file.texts.get(lang).and_then(|t| t.rules.get(&rule.id)) {
        Some((title, summary)) => (title.as_str(), summary.as_str()),
        None => (rule.title.as_str(), rule.summary.as_str()),
    }
}

/// The rules that hold every word of the search and the groups above them.
fn matches(file: &CatalogFile, lang: &str, words: &[String]) -> (HashSet<String>, HashSet<String>) {
    let mut rules = HashSet::new();
    let mut groups = HashSet::new();
    for rule in &file.rules {
        let (title, summary) = rule_texts(file, lang, rule);
        let hay = format!("{} {} {} {}", rule.id, rule.title, title, summary).to_lowercase();
        if !words.iter().all(|w| hay.contains(w.as_str())) {
            continue;
        }
        rules.insert(rule.id.clone());
        let mut next = Some(rule.group.clone());
        while let Some(g) = next {
            if !groups.insert(g.clone()) {
                break;
            }
            next = file
                .groups
                .iter()
                .find(|x| x.id == g)
                .and_then(|x| x.parent.clone());
        }
    }
    (rules, groups)
}

/// What the search lets through: everything, or the matching rules and their groups.
struct Filter {
    words: bool,
    rules: HashSet<String>,
    groups: HashSet<String>,
}

/// The rows of the tree under a group: groups as `g:<id>`, policies as `r:<id>`.
fn catalog_rows(
    rows: &mut Vec<tree::Row<String>>,
    file: &CatalogFile,
    lang: &str,
    doc: &CatalogDoc,
    filter: &Filter,
    group: &Group,
    depth: usize,
) {
    if filter.words && !filter.groups.contains(&group.id) {
        return;
    }
    let open = filter.words || doc.expanded.contains(&group.id);
    let rules: Vec<&Rule> = file
        .rules
        .iter()
        .filter(|r| r.group == group.id)
        .filter(|r| !filter.words || filter.rules.contains(&r.id))
        .collect();
    rows.push(tree::Row {
        key: format!("g:{}", group.id),
        depth,
        title: group_title(file, lang, group).to_owned(),
        right: rules.len().to_string(),
        check: tree::Check::None,
        open: Some(open),
        muted: false,
        icon: None,
        switch: None,
    });
    if !open {
        return;
    }
    for child in file
        .groups
        .iter()
        .filter(|g| g.parent.as_deref() == Some(group.id.as_str()))
    {
        catalog_rows(rows, file, lang, doc, filter, child, depth + 1);
    }
    for rule in rules {
        rows.push(tree::Row {
            key: format!("r:{}", rule.id),
            depth: depth + 1,
            title: rule_texts(file, lang, rule).0.to_owned(),
            right: String::new(),
            check: tree::Check::None,
            open: None,
            muted: false,
            icon: None,
            switch: None,
        });
    }
}

/// The tab: the tree on the left, the selected policy on the right.
pub fn catalog_view(ui: &mut egui::Ui, file: &CatalogFile, doc: &CatalogDoc, acts: &mut Vec<Act>) {
    let lang = crate::i18n::current();
    let words = doc.search.words();
    let (rules, groups) = matches(file, &lang, &words);
    let filter = Filter {
        words: !words.is_empty(),
        rules,
        groups,
    };
    let mut rows = Vec::new();
    for root in roots(file) {
        catalog_rows(&mut rows, file, &lang, doc, &filter, root, 0);
    }
    let key = match (&doc.sel, &doc.group) {
        (Some(r), _) => Some(format!("r:{r}")),
        (None, Some(g)) => Some(format!("g:{g}")),
        _ => None,
    };
    let selected = key.and_then(|k| rows.iter().position(|r| r.key == k));
    let scroll = if doc.reveal {
        tree::Scroll::Center
    } else if doc.follow {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    if selected.is_none() && scroll != tree::Scroll::None {
        acts.push(Act::Revealed);
    }
    let p = style::palette(ui);
    egui::Panel::left("catalog-tree")
        .resizable(true)
        .default_size(380.0)
        .frame(egui::Frame::new().fill(p.card).inner_margin(egui::Margin::symmetric(0, 4)))
        .show(ui, |ui| {
            if filter.words && filter.rules.is_empty() {
                ui.weak(tr("Nothing matches the search."));
            }
            let id = egui::Id::new(("catalog-tree", doc.id.as_str()));
            let out = tree::show(ui, id, &rows, selected, scroll);
            for event in out.events {
                match event {
                    tree::Event::Select(k) => {
                        let key = k.strip_prefix("r:").map_or(k.clone(), str::to_owned);
                        acts.push(Act::CatalogSelect(key));
                    }
                    tree::Event::Expand(k, open) => {
                        if let Some(g) = k.strip_prefix("g:") {
                            acts.push(Act::CatalogExpand(g.to_owned(), open));
                        }
                    }
                    tree::Event::Toggle(_) | tree::Event::Switch(..) => {}
                }
            }
            if out.moved {
                acts.push(Act::Follow);
            }
            if out.scrolled {
                acts.push(Act::Revealed);
            }
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| detail(ui, file, &lang, doc, acts));
    });
}

fn detail(
    ui: &mut egui::Ui,
    file: &CatalogFile,
    lang: &str,
    doc: &CatalogDoc,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = doc
        .sel
        .as_deref()
        .and_then(|id| file.rules.iter().find(|r| r.id == id))
    else {
        if let Some(group) = doc
            .group
            .as_deref()
            .and_then(|g| file.groups.iter().find(|x| x.id == g))
        {
            ui.heading(group_title(file, lang, group));
            crate::views::id_line(ui, &group.id);
            ui.weak(trf(
                "{n} policies",
                &[("n", &file.rules.iter().filter(|r| r.group == group.id).count())],
            ));
            return;
        }
        ui.heading(&file.name);
        ui.weak(trf(
            "{rules} rules, {skipped} skipped",
            &[("rules", &file.rules.len()), ("skipped", &file.skipped.len())],
        ));
        if let Some(path) = &doc.path {
            ui.weak(trf(
                "Opened from {path}, not imported",
                &[("path", &path.display())],
            ));
        }
        ui.add_space(6.0);
        ui.label(tr("Select a policy in the tree."));
        return;
    };
    let (title, summary) = rule_texts(file, lang, rule);
    ui.label(RichText::new(title).size(18.0).strong());
    crate::views::id_line(ui, &rule.id);
    ui.add_space(6.0);
    if doc.file.is_none() && ui.button(tr("Show in the profile tree")).clicked() {
        acts.push(Act::ShowInProfile(file.id.clone(), rule.id.clone()));
    }
    if !summary.is_empty() {
        ui.add_space(6.0);
        ui.label(summary);
    }
    if !rule.params.is_empty() {
        ui.add_space(6.0);
        ui.strong(tr("Parameters"));
        for (name, param) in &rule.params {
            ui.label(format!("{name}: {}", param.title()));
        }
    }
    ui.add_space(6.0);
    ui.strong(tr("What it writes"));
    for action in &rule.actions {
        ui.monospace(crate::views::describe(action));
    }
}
