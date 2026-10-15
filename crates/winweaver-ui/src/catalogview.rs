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
use crate::{Act, Search, style};

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
    pub expanded: HashSet<String>,
    pub search: Search,
    /// The selected row is scrolled into view at the next frame.
    pub reveal: bool,
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
            expanded: roots(file).map(|g| g.id.clone()).collect(),
            search: Search::default(),
            reveal: false,
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
    let p = style::palette(ui);
    egui::Panel::left("catalog-tree")
        .resizable(true)
        .default_size(380.0)
        .frame(egui::Frame::new().fill(p.card).inner_margin(egui::Margin::symmetric(6, 4)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    if filter.words && filter.rules.is_empty() {
                        ui.weak(tr("Nothing matches the search."));
                    }
                    for root in roots(file) {
                        group_rows(ui, file, &lang, doc, &filter, root, acts);
                    }
                });
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| detail(ui, file, &lang, doc, acts));
    });
}

fn group_rows(
    ui: &mut egui::Ui,
    file: &CatalogFile,
    lang: &str,
    doc: &CatalogDoc,
    filter: &Filter,
    group: &Group,
    acts: &mut Vec<Act>,
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
    ui.horizontal(|ui| {
        let twisty = if open { "\u{25BE}" } else { "\u{25B8}" };
        if ui
            .add(egui::Button::new(style::muted(ui, twisty, 11.0)).frame(false))
            .clicked()
        {
            acts.push(Act::CatalogExpand(group.id.clone(), !open));
        }
        ui.label(group_title(file, lang, group));
        ui.label(style::muted(ui, rules.len().to_string(), 11.0));
    });
    if !open {
        return;
    }
    ui.indent(&group.id, |ui| {
        for child in file
            .groups
            .iter()
            .filter(|g| g.parent.as_deref() == Some(group.id.as_str()))
        {
            group_rows(ui, file, lang, doc, filter, child, acts);
        }
        for rule in rules {
            let selected = doc.sel.as_deref() == Some(rule.id.as_str());
            let (title, _) = rule_texts(file, lang, rule);
            let response = ui.selectable_label(selected, title);
            if selected && doc.reveal {
                response.scroll_to_me(Some(egui::Align::Center));
                acts.push(Act::Revealed);
            }
            if response.clicked() {
                acts.push(Act::CatalogSelect(rule.id.clone()));
            }
        }
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
    ui.monospace(&rule.id);
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
