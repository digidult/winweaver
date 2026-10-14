//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`), built on eframe.
//!
//! This is the skeleton of variant A (the command bar): the tab row with the menu button, the
//! command bar, the navigation row (back, forward, breadcrumb, bookmark, search), the profile
//! tree with the detail panel, the Settings tab and the status bar. Every change of the profile
//! is collected as an [`Act`] while the frame is drawn and applied afterwards.

mod views;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::{Catalog, TREE_ROOTS, check};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::profile::Profile;
use winweaver_core::resolver::{self, Resolution, State};

/// The presets embedded in the executable: the name and the profile text.
pub const PRESETS: [(&str, &str); 4] = [
    (
        "Office",
        include_str!("../../../data/presets/office.wwprofile"),
    ),
    (
        "Strict",
        include_str!("../../../data/presets/strict.wwprofile"),
    ),
    (
        "Laptop",
        include_str!("../../../data/presets/laptop.wwprofile"),
    ),
    ("Home", include_str!("../../../data/presets/home.wwprofile")),
];

const SEARCH_HISTORY: usize = 20;

/// The selected item of a profile document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sel {
    None,
    Group(String),
    Rule(String),
}

/// An open profile with its view state.
pub struct ProfileDoc {
    pub profile: Profile,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub sel: Sel,
    history: Vec<Sel>,
    pos: usize,
    pub expanded: HashSet<String>,
    pub resolution: Resolution,
}

impl ProfileDoc {
    pub fn new(profile: Profile, path: Option<PathBuf>, catalog: &Catalog) -> Self {
        let resolution = resolver::resolve(catalog, &profile);
        let expanded = TREE_ROOTS.iter().map(|s| (*s).to_owned()).collect();
        Self {
            profile,
            path,
            dirty: false,
            sel: Sel::None,
            history: vec![Sel::None],
            pos: 0,
            expanded,
            resolution,
        }
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => format!("{}.wwprofile", self.profile.name),
        }
    }

    fn select(&mut self, sel: Sel) {
        if self.sel == sel {
            return;
        }
        self.history.truncate(self.pos + 1);
        self.history.push(sel.clone());
        self.pos = self.history.len() - 1;
        self.sel = sel;
    }

    fn can_go(&self, delta: isize) -> bool {
        let n = self.pos as isize + delta;
        n >= 0 && (n as usize) < self.history.len()
    }

    fn go(&mut self, delta: isize) {
        if self.can_go(delta) {
            self.pos = (self.pos as isize + delta) as usize;
            self.sel = self.history[self.pos].clone();
        }
    }

    fn refresh(&mut self, catalog: &Catalog) {
        self.resolution = resolver::resolve(catalog, &self.profile);
        self.dirty = true;
    }
}

/// A tab of the window.
pub enum Doc {
    Profile(Box<ProfileDoc>),
    Settings,
}

impl Doc {
    pub fn title(&self) -> String {
        match self {
            Doc::Profile(d) => d.title(),
            Doc::Settings => "Settings".to_owned(),
        }
    }

    pub fn dirty(&self) -> bool {
        matches!(self, Doc::Profile(d) if d.dirty)
    }
}

/// The search of the navigation row.
#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub text: String,
    pub history: Vec<String>,
    focus: bool,
}

impl Search {
    /// The lower case words of the search text.
    pub fn words(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect()
    }

    fn remember(&mut self) {
        let text = self.text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.history.retain(|h| h != &text);
        self.history.insert(0, text);
        self.history.truncate(SEARCH_HISTORY);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::System => egui::ThemePreference::System,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
        }
    }
}

pub struct Settings {
    /// The language code of the catalog texts: `en` or a code of the translations.
    pub language: String,
    pub theme: ThemeChoice,
    /// Whether the description panel is shown.
    pub desc_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".to_owned(),
            theme: ThemeChoice::System,
            desc_open: true,
        }
    }
}

/// A change requested while a frame was drawn, applied after it.
pub enum Act {
    Select(Sel),
    Toggle(String, bool),
    SetGroup(String, bool),
    SetParam(String, String, Value),
    Expand(String, bool),
    Activate(usize),
    CloseTab(usize, bool),
    NewProfile(Option<&'static str>),
    OpenSettings,
    Save,
    Back,
    Forward,
    ToggleBookmark,
    Fold,
    Check,
    Exit,
    Message(String),
}

pub struct App {
    catalog: Catalog,
    texts: Vec<CatalogTexts>,
    docs: Vec<Doc>,
    active: usize,
    settings: Settings,
    search: Search,
    status: String,
    bookmarks: HashSet<String>,
    theme_applied: bool,
    exit: bool,
}

/// The translation for a language code, `None` for English.
pub fn texts_for<'a>(texts: &'a [CatalogTexts], code: &str) -> Option<&'a CatalogTexts> {
    texts.iter().find(|t| t.code == code)
}

impl App {
    pub fn new(catalog: Catalog, texts: Vec<CatalogTexts>) -> Self {
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0)
            .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &catalog));
        let doc = ProfileDoc::new(profile, None, &catalog);
        Self {
            catalog,
            texts,
            docs: vec![Doc::Profile(Box::new(doc))],
            active: 0,
            settings: Settings::default(),
            search: Search::default(),
            status: "Ready".to_owned(),
            bookmarks: HashSet::new(),
            theme_applied: false,
            exit: false,
        }
    }

    fn active_profile(&mut self) -> Option<&mut ProfileDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Profile(doc)) => Some(doc),
            _ => None,
        }
    }

    fn refresh_active(&mut self) {
        let catalog = &self.catalog;
        if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
            doc.refresh(catalog);
        }
    }

    fn bookmark_key(&self) -> Option<String> {
        match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => {
                let item = match &doc.sel {
                    Sel::None => String::new(),
                    Sel::Group(id) | Sel::Rule(id) => id.clone(),
                };
                Some(format!("{}:{item}", doc.title()))
            }
            _ => None,
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let mut acts = Vec::new();
        let active = self.active;
        let count = self.docs.len();
        ctx.input_mut(|i| {
            let sc = |m, k| KeyboardShortcut::new(m, k);
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::F)) {
                self.search.open = true;
                self.search.focus = true;
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::D)) {
                acts.push(Act::Fold);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::S)) {
                acts.push(Act::Save);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::N)) {
                acts.push(Act::NewProfile(None));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::B)) {
                acts.push(Act::ToggleBookmark);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::W)) {
                acts.push(Act::CloseTab(active, false));
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::Comma)) {
                acts.push(Act::OpenSettings);
            }
            if i.consume_shortcut(&sc(Modifiers::NONE, Key::F7)) {
                acts.push(Act::Check);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowLeft)) {
                acts.push(Act::Back);
            }
            if i.consume_shortcut(&sc(Modifiers::ALT, Key::ArrowRight)) {
                acts.push(Act::Forward);
            }
            if count > 1 && i.consume_shortcut(&sc(Modifiers::CTRL, Key::Tab)) {
                acts.push(Act::Activate((active + 1) % count));
            }
        });
        acts
    }

    fn tabs_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        ui.horizontal(|ui| {
            ui.menu_button("Menu", |ui| {
                ui.menu_button("New", |ui| {
                    if ui.button("Empty profile").clicked() {
                        acts.push(Act::NewProfile(None));
                        ui.close();
                    }
                    ui.separator();
                    for (name, _) in PRESETS {
                        if ui
                            .button(format!("Profile from the preset {name}"))
                            .clicked()
                        {
                            acts.push(Act::NewProfile(Some(name)));
                            ui.close();
                        }
                    }
                });
                if ui
                    .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    acts.push(Act::Save);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Close tab").shortcut_text("Ctrl+W"))
                    .clicked()
                {
                    acts.push(Act::CloseTab(self.active, false));
                    ui.close();
                }
                if ui.button("Close tab, discard changes").clicked() {
                    acts.push(Act::CloseTab(self.active, true));
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Settings").shortcut_text("Ctrl+,"))
                    .clicked()
                {
                    acts.push(Act::OpenSettings);
                    ui.close();
                }
                if ui.button("About").clicked() {
                    acts.push(Act::Message(format!(
                        "WinWeaver {}, catalog {}",
                        winweaver_core::APP_VERSION,
                        self.catalog.version()
                    )));
                    ui.close();
                }
                if ui.button("Exit").clicked() {
                    acts.push(Act::Exit);
                    ui.close();
                }
            });
            ui.separator();
            for (i, doc) in self.docs.iter().enumerate() {
                let mark = if doc.dirty() { "* " } else { "" };
                let label = format!("{mark}{}", doc.title());
                if ui.selectable_label(i == self.active, label).clicked() {
                    acts.push(Act::Activate(i));
                }
                if ui
                    .small_button("x")
                    .on_hover_text("Close (Ctrl+W)")
                    .clicked()
                {
                    acts.push(Act::CloseTab(i, false));
                }
                ui.add_space(4.0);
            }
            if ui
                .button("+")
                .on_hover_text("New profile (Ctrl+N)")
                .clicked()
            {
                acts.push(Act::NewProfile(None));
            }
        });
    }

    fn toolbar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                acts.push(Act::NewProfile(None));
            }
            if ui.button("Open").clicked() {
                acts.push(Act::Message(
                    "Open: the file dialog comes with the next milestone".into(),
                ));
            }
            if ui.button("Save").on_hover_text("Ctrl+S").clicked() {
                acts.push(Act::Save);
            }
            ui.separator();
            if ui.button("Check").on_hover_text("F7").clicked() {
                acts.push(Act::Check);
            }
            if ui.button("Build").on_hover_text("F9").clicked() {
                acts.push(Act::Message(
                    "Build: the renderer comes with the next milestone".into(),
                ));
            }
            ui.separator();
            if ui.button("This PC").clicked() {
                acts.push(Act::Message(
                    "This PC: the native read comes with the next milestone".into(),
                ));
            }
            if ui.button("Compare").clicked() {
                acts.push(Act::Message(
                    "Compare: the diff view comes with the next milestone".into(),
                ));
            }
        });
    }

    fn crumbs(&self) -> Vec<(String, Sel)> {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return Vec::new();
        };
        let texts = texts_for(&self.texts, &self.settings.language);
        let mut crumbs = vec![(doc.profile.name.clone(), Sel::None)];
        let group = match &doc.sel {
            Sel::None => None,
            Sel::Group(id) => Some(id.clone()),
            Sel::Rule(id) => self.catalog.rule(id).map(|r| r.group.clone()),
        };
        if let Some(group) = group {
            for id in self.catalog.path_of(&group) {
                if let Some(g) = self.catalog.group(id) {
                    crumbs.push((
                        views::gtitle(texts, g).to_owned(),
                        Sel::Group(id.to_owned()),
                    ));
                }
            }
        }
        if let Sel::Rule(id) = &doc.sel
            && let Some(rule) = self.catalog.rule(id)
        {
            crumbs.push((views::rtitle(texts, rule).to_owned(), Sel::Rule(id.clone())));
        }
        crumbs
    }

    fn nav_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let (can_back, can_forward) = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => (doc.can_go(-1), doc.can_go(1)),
            _ => (false, false),
        };
        let crumbs = self.crumbs();
        let marked = self
            .bookmark_key()
            .is_some_and(|k| self.bookmarks.contains(&k));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_back, egui::Button::new("<").frame(false))
                .on_hover_text("Back (Alt+Left)")
                .clicked()
            {
                acts.push(Act::Back);
            }
            if ui
                .add_enabled(can_forward, egui::Button::new(">").frame(false))
                .on_hover_text("Forward (Alt+Right)")
                .clicked()
            {
                acts.push(Act::Forward);
            }
            ui.separator();
            for (i, (label, sel)) in crumbs.iter().enumerate() {
                if i > 0 {
                    ui.weak(">");
                }
                let text = if i + 1 == crumbs.len() {
                    egui::RichText::new(label.as_str()).strong()
                } else {
                    egui::RichText::new(label.as_str())
                };
                if ui.add(egui::Button::new(text).frame(false)).clicked() {
                    acts.push(Act::Select(sel.clone()));
                }
            }
            if !crumbs.is_empty() {
                let mut m = marked;
                if ui
                    .toggle_value(&mut m, "Bookmark")
                    .on_hover_text("Ctrl+B")
                    .changed()
                {
                    acts.push(Act::ToggleBookmark);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.search.open {
                    if ui
                        .button("x")
                        .on_hover_text("Clear and close (Esc)")
                        .clicked()
                    {
                        self.search.text.clear();
                        self.search.open = false;
                    }
                    let id = egui::Id::new("search-field");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search.text)
                            .id(id)
                            .hint_text("Search rules (Ctrl+F)")
                            .desired_width(280.0),
                    );
                    if self.search.focus {
                        ui.memory_mut(|m| m.request_focus(id));
                        self.search.focus = false;
                    }
                    if response.lost_focus() {
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.search.text.clear();
                            self.search.open = false;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.search.remember();
                        }
                    }
                    if !self.search.history.is_empty() {
                        ui.menu_button("v", |ui| {
                            for h in self.search.history.clone() {
                                if ui.button(&h).clicked() {
                                    self.search.text = h;
                                    ui.close();
                                }
                            }
                        });
                    }
                } else if ui.button("Search").on_hover_text("Ctrl+F").clicked() {
                    self.search.open = true;
                    self.search.focus = true;
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let state = match self.docs.get(self.active) {
                Some(Doc::Profile(d)) if d.dirty => "Modified",
                Some(Doc::Profile(_)) => "Saved",
                _ => "",
            };
            ui.weak(state);
            ui.weak(format!(
                "Catalog {}, {} rules",
                self.catalog.version(),
                self.catalog.rules().len()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(&self.status);
            });
        });
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Language of the catalog texts");
            let current = if self.settings.language == "en" {
                "English".to_owned()
            } else {
                texts_for(&self.texts, &self.settings.language)
                    .map(|t| t.language.clone())
                    .unwrap_or_else(|| self.settings.language.clone())
            };
            egui::ComboBox::from_id_salt("language")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.language, "en".to_owned(), "English");
                    for t in &self.texts {
                        ui.selectable_value(
                            &mut self.settings.language,
                            t.code.clone(),
                            &t.language,
                        );
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Theme");
            let before = self.settings.theme;
            ui.radio_value(
                &mut self.settings.theme,
                ThemeChoice::System,
                "Follow Windows",
            );
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Light, "Light");
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Dark, "Dark");
            if before != self.settings.theme {
                self.theme_applied = false;
            }
        });
        ui.checkbox(
            &mut self.settings.desc_open,
            "Show the description panel (Ctrl+D)",
        );
        ui.add_space(12.0);
        ui.weak("Settings are not saved yet; settings.json next to the executable comes with the next milestone.");
    }

    fn save_active(&mut self) {
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            self.status = "Nothing to save".to_owned();
            return;
        };
        let path = match &doc.path {
            Some(p) => p.clone(),
            None => profiles_dir().join(format!("{}.wwprofile", file_stem(&doc.profile.name))),
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match doc.profile.save(&path) {
            Ok(()) => {
                doc.path = Some(path.clone());
                doc.dirty = false;
                self.status = format!("Saved {}", path.display());
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn check_active(&mut self) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let blocked = doc
            .resolution
            .states
            .values()
            .filter(|s| matches!(s, State::Blocked { .. }))
            .count();
        let conflicts = doc.resolution.conflicts.len();
        let problems = check::check(&self.catalog).len();
        self.status = format!(
            "Check: {conflicts} conflicts, {blocked} rules blocked by their dependencies, {problems} catalog problems"
        );
    }

    fn apply(&mut self, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Select(sel) => {
                    if let Some(doc) = self.active_profile() {
                        doc.select(sel);
                    }
                }
                Act::Toggle(id, on) => {
                    if let Some(doc) = self.active_profile() {
                        doc.profile.set_on(&id, on);
                    }
                    self.refresh_active();
                }
                Act::SetGroup(group, on) => {
                    let ids: Vec<String> = self
                        .catalog
                        .rules_under(&group)
                        .iter()
                        .map(|r| r.id.clone())
                        .collect();
                    if let Some(doc) = self.active_profile() {
                        for id in &ids {
                            doc.profile.set_on(id, on);
                        }
                    }
                    self.refresh_active();
                }
                Act::SetParam(id, name, value) => {
                    let default = self
                        .catalog
                        .rule(&id)
                        .and_then(|r| r.params.get(&name))
                        .map(|p| p.default_value());
                    if let Some(doc) = self.active_profile() {
                        let choice = doc.profile.rules.entry(id).or_default();
                        if default
                            .as_ref()
                            .is_some_and(|d| check::same_value(d, &value))
                        {
                            choice.params.shift_remove(&name);
                        } else {
                            choice.params.insert(name, value);
                        }
                    }
                    self.refresh_active();
                }
                Act::Expand(group, open) => {
                    if let Some(doc) = self.active_profile() {
                        if open {
                            doc.expanded.insert(group);
                        } else {
                            doc.expanded.remove(&group);
                        }
                    }
                }
                Act::Activate(i) => {
                    if i < self.docs.len() {
                        self.active = i;
                    }
                }
                Act::CloseTab(i, force) => {
                    if i < self.docs.len() {
                        if self.docs[i].dirty() && !force {
                            self.status =
                                "The tab has unsaved changes: save it (Ctrl+S) or close it from the menu discarding them".to_owned();
                        } else {
                            self.docs.remove(i);
                            if self.active >= self.docs.len() {
                                self.active = self.docs.len().saturating_sub(1);
                            }
                        }
                    }
                }
                Act::NewProfile(preset) => {
                    let profile = match preset.and_then(|n| PRESETS.iter().find(|p| p.0 == n)) {
                        Some((name, text)) => Profile::parse(text, name)
                            .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog)),
                        None => Profile::new("New profile", self.catalog.version()),
                    };
                    let doc = ProfileDoc::new(profile, None, &self.catalog);
                    self.docs.push(Doc::Profile(Box::new(doc)));
                    self.active = self.docs.len() - 1;
                }
                Act::OpenSettings => {
                    match self.docs.iter().position(|d| matches!(d, Doc::Settings)) {
                        Some(i) => self.active = i,
                        None => {
                            self.docs.push(Doc::Settings);
                            self.active = self.docs.len() - 1;
                        }
                    }
                }
                Act::Save => self.save_active(),
                Act::Back => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(-1);
                    }
                }
                Act::Forward => {
                    if let Some(doc) = self.active_profile() {
                        doc.go(1);
                    }
                }
                Act::ToggleBookmark => {
                    if let Some(key) = self.bookmark_key() {
                        if !self.bookmarks.remove(&key) {
                            self.bookmarks.insert(key);
                        }
                        self.status = format!(
                            "{} bookmarks (kept in memory for now)",
                            self.bookmarks.len()
                        );
                    }
                }
                Act::Fold => self.settings.desc_open = !self.settings.desc_open,
                Act::Check => self.check_active(),
                Act::Exit => self.exit = true,
                Act::Message(text) => self.status = text,
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.exit {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.theme_applied {
            ui.ctx().set_theme(self.settings.theme.preference());
            self.theme_applied = true;
        }
        let mut acts = self.shortcuts(ui.ctx());
        egui::Panel::top("tabs").show(ui, |ui| self.tabs_row(ui, &mut acts));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui, &mut acts));
        egui::Panel::top("nav").show(ui, |ui| self.nav_row(ui, &mut acts));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        let kind = match self.docs.get(self.active) {
            Some(Doc::Profile(_)) => 0,
            Some(Doc::Settings) => 1,
            None => 2,
        };
        if kind == 0 {
            let catalog = &self.catalog;
            let texts = texts_for(&self.texts, &self.settings.language);
            let search = &self.search;
            let settings = &self.settings;
            if let Some(Doc::Profile(doc)) = self.docs.get(self.active) {
                let doc: &ProfileDoc = doc;
                egui::Panel::left("tree")
                    .resizable(true)
                    .default_size(380.0)
                    .show(ui, |ui| {
                        views::tree_view(ui, catalog, texts, doc, search, &mut acts)
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    views::detail_view(ui, catalog, texts, doc, settings, &mut acts);
                });
            }
        } else if kind == 1 {
            egui::CentralPanel::default().show(ui, |ui| self.settings_view(ui));
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("No document is open. Use Menu, New or Ctrl+N.");
            });
        }
        self.apply(acts);
    }
}

/// The folder of the user's profiles next to the executable.
pub fn profiles_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("profiles")
}

/// A file name stem made of a profile name.
pub fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim().to_owned();
    if stem.is_empty() {
        "profile".to_owned()
    } else {
        stem
    }
}

/// Opens the window; returns the error text when the catalog or the window cannot start.
pub fn run() -> Result<(), String> {
    let catalog = Catalog::embedded().map_err(|e| format!("catalog: {e}"))?;
    let texts = CatalogTexts::embedded().map_err(|e| format!("translations: {e}"))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WinWeaver")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "WinWeaver",
        options,
        Box::new(move |_cc| Ok(Box::new(App::new(catalog, texts)))),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems_are_safe() {
        assert_eq!(file_stem("Office 2026"), "Office 2026");
        assert_eq!(file_stem("a/b:c"), "a_b_c");
        assert_eq!(file_stem("   "), "profile");
    }

    #[test]
    fn app_starts_with_the_office_preset() {
        let catalog = Catalog::embedded().unwrap();
        let app = App::new(catalog, CatalogTexts::embedded().unwrap());
        assert_eq!(app.docs.len(), 1);
        assert_eq!(app.docs[0].title(), "Office.wwprofile");
        assert!(!app.docs[0].dirty());
    }

    #[test]
    fn history_moves_back_and_forward() {
        let catalog = Catalog::embedded().unwrap();
        let mut doc = ProfileDoc::new(Profile::from_defaults("x", &catalog), None, &catalog);
        doc.select(Sel::Group("policies".into()));
        doc.select(Sel::Rule("install.bypass-tpm".into()));
        assert!(doc.can_go(-1) && !doc.can_go(1));
        doc.go(-1);
        assert_eq!(doc.sel, Sel::Group("policies".into()));
        doc.go(1);
        assert_eq!(doc.sel, Sel::Rule("install.bypass-tpm".into()));
    }
}
