//! The WinWeaver window: one window with document tabs (`docs/spec/01-ui.md`), built on eframe.
//!
//! This is the skeleton of variant A (the command bar): the tab row with the menu button, the
//! command bar, the navigation row (back, forward, breadcrumb, bookmark, search), the profile
//! tree with the detail panel, the Settings tab and the status bar. Every change of the profile
//! is collected as an [`Act`] while the frame is drawn and applied afterwards.

mod settings;
mod views;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui;
use serde_json::Value;
use winweaver_core::catalog::{Catalog, TREE_ROOTS, check};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::profile::{self, Profile};
use winweaver_core::resolver::{self, Resolution, State};

pub use settings::{Settings, ThemeChoice, exe_dir};

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
            None => format!("{}.{}", self.profile.name, profile::EXTENSION),
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

    /// Puts the current text first in the history; `true` when the history changed.
    fn remember(&mut self) -> bool {
        let text = self.text.trim().to_owned();
        if text.is_empty() || self.history.first() == Some(&text) {
            return false;
        }
        self.history.retain(|h| h != &text);
        self.history.insert(0, text);
        self.history.truncate(SEARCH_HISTORY);
        true
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
    Open,
    OpenPath(PathBuf),
    OpenSettings,
    Save,
    SaveAs,
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
    settings_dirty: bool,
    search: Search,
    status: String,
    bookmarks: HashSet<String>,
    theme_applied: bool,
    last_title: String,
    exit: bool,
}

/// The translation for a language code, `None` for English.
pub fn texts_for<'a>(texts: &'a [CatalogTexts], code: &str) -> Option<&'a CatalogTexts> {
    texts.iter().find(|t| t.code == code)
}

impl App {
    pub fn new(catalog: Catalog, texts: Vec<CatalogTexts>, settings: Settings) -> Self {
        let profile = Profile::parse(PRESETS[0].1, PRESETS[0].0)
            .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &catalog));
        let doc = ProfileDoc::new(profile, None, &catalog);
        let search = Search {
            history: settings.search_history.clone(),
            ..Search::default()
        };
        Self {
            catalog,
            texts,
            docs: vec![Doc::Profile(Box::new(doc))],
            active: 0,
            settings,
            settings_dirty: false,
            search,
            status: "Ready".to_owned(),
            bookmarks: HashSet::new(),
            theme_applied: false,
            last_title: String::new(),
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
            if i.consume_shortcut(&sc(Modifiers::CTRL | Modifiers::SHIFT, Key::S)) {
                acts.push(Act::SaveAs);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::S)) {
                acts.push(Act::Save);
            }
            if i.consume_shortcut(&sc(Modifiers::CTRL, Key::O)) {
                acts.push(Act::Open);
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
            for file in &i.raw.dropped_files {
                acts.push(Act::OpenPath(file.path().to_path_buf()));
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
                    .add(egui::Button::new("Open...").shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    acts.push(Act::Open);
                    ui.close();
                }
                ui.add_enabled_ui(!self.settings.recent.is_empty(), |ui| {
                    ui.menu_button("Open recent", |ui| {
                        for path in &self.settings.recent {
                            if ui.button(path.display().to_string()).clicked() {
                                acts.push(Act::OpenPath(path.clone()));
                                ui.close();
                            }
                        }
                    });
                });
                if ui
                    .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    acts.push(Act::Save);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Save as...").shortcut_text("Ctrl+Shift+S"))
                    .clicked()
                {
                    acts.push(Act::SaveAs);
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
                        "WinWeaver {}, catalog {}, settings {}",
                        winweaver_core::APP_VERSION,
                        self.catalog.version(),
                        Settings::path().display()
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
                let response = ui.selectable_label(i == self.active, label);
                if response.clicked() {
                    acts.push(Act::Activate(i));
                }
                if response.middle_clicked() {
                    acts.push(Act::CloseTab(i, false));
                }
                if let Doc::Profile(d) = doc
                    && let Some(p) = &d.path
                {
                    response.on_hover_text(p.display().to_string());
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
            if ui.button("New").on_hover_text("Ctrl+N").clicked() {
                acts.push(Act::NewProfile(None));
            }
            if ui.button("Open").on_hover_text("Ctrl+O").clicked() {
                acts.push(Act::Open);
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
                        } else if ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && self.search.remember()
                        {
                            self.settings.search_history = self.search.history.clone();
                            self.settings_dirty = true;
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
                Some(Doc::Profile(d)) if d.path.is_some() => "Saved",
                Some(Doc::Profile(_)) => "Not saved yet",
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
        let before = self.settings.clone();
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
            ui.radio_value(
                &mut self.settings.theme,
                ThemeChoice::System,
                "Follow Windows",
            );
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Light, "Light");
            ui.radio_value(&mut self.settings.theme, ThemeChoice::Dark, "Dark");
        });
        ui.checkbox(
            &mut self.settings.desc_open,
            "Show the description panel (Ctrl+D)",
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.weak(format!("Settings file: {}", Settings::path().display()));
            if !self.settings.recent.is_empty() && ui.small_button("Forget recent files").clicked()
            {
                self.settings.recent.clear();
            }
        });
        if before != self.settings {
            self.settings_dirty = true;
            if before.theme != self.settings.theme {
                self.theme_applied = false;
            }
        }
    }

    fn remember_recent(&mut self, path: &Path) {
        self.settings.remember(path);
        self.settings_dirty = true;
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| matches!(d, Doc::Profile(p) if p.path.as_ref() == Some(&path)))
        {
            self.active = i;
            return;
        }
        match Profile::load(&path) {
            Ok(mut profile) => {
                let unknown = profile.split_unknown(&self.catalog);
                let doc = ProfileDoc::new(profile, Some(path.clone()), &self.catalog);
                self.docs.push(Doc::Profile(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = if unknown.is_empty() {
                    format!("Opened {}", path.display())
                } else {
                    format!(
                        "Opened {} ({} choices of rules this catalog does not have are kept aside)",
                        path.display(),
                        unknown.len()
                    )
                };
                self.remember_recent(&path);
            }
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }

    fn save_active(&mut self, ask: bool) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = "Nothing to save: the active tab is not a profile".to_owned();
            return;
        };
        let path = match (&doc.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let proposed = format!("{}.{}", file_stem(&doc.profile.name), profile::EXTENSION);
                match ask_save_path(&proposed) {
                    Some(p) => with_extension(p),
                    None => return,
                }
            }
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) else {
            return;
        };
        match doc.profile.save(&path) {
            Ok(()) => {
                doc.path = Some(path.clone());
                doc.dirty = false;
                self.status = format!("Saved {}", path.display());
                self.remember_recent(&path);
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
                            self.status = "The tab has unsaved changes: save it (Ctrl+S) or close it from the menu discarding them".to_owned();
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
                Act::Open => {
                    if let Some(path) = pick_profile() {
                        self.open_path(path);
                    }
                }
                Act::OpenPath(path) => self.open_path(path),
                Act::OpenSettings => {
                    match self.docs.iter().position(|d| matches!(d, Doc::Settings)) {
                        Some(i) => self.active = i,
                        None => {
                            self.docs.push(Doc::Settings);
                            self.active = self.docs.len() - 1;
                        }
                    }
                }
                Act::Save => self.save_active(false),
                Act::SaveAs => self.save_active(true),
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
                Act::Fold => {
                    self.settings.desc_open = !self.settings.desc_open;
                    self.settings_dirty = true;
                }
                Act::Check => self.check_active(),
                Act::Exit => self.exit = true,
                Act::Message(text) => self.status = text,
            }
        }
    }

    fn persist_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        if let Err(e) = self.settings.save_to(&Settings::path()) {
            self.status = format!("Settings not saved: {e}");
        }
    }

    fn window_title(&self) -> String {
        match self.docs.get(self.active) {
            Some(doc) => format!("{} - WinWeaver", doc.title()),
            None => "WinWeaver".to_owned(),
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
        let title = self.window_title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
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
                        views::tree_view(ui, catalog, texts, doc, search, &mut acts);
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
        self.persist_settings();
    }
}

/// The folder of the user's profiles next to the executable.
pub fn profiles_dir() -> PathBuf {
    exe_dir().join("profiles")
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

/// The path with the profile extension when it has none.
fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(profile::EXTENSION)
    }
}

fn profile_dialog() -> rfd::FileDialog {
    let dir = profiles_dir();
    let dialog = rfd::FileDialog::new().add_filter("WinWeaver profile", &[profile::EXTENSION]);
    if dir.is_dir() {
        dialog.set_directory(dir)
    } else {
        dialog
    }
}

fn pick_profile() -> Option<PathBuf> {
    profile_dialog().set_title("Open a profile").pick_file()
}

fn ask_save_path(proposed: &str) -> Option<PathBuf> {
    let _ = std::fs::create_dir_all(profiles_dir());
    profile_dialog()
        .set_title("Save the profile as")
        .set_file_name(proposed)
        .save_file()
}

/// Opens the window; returns the error text when the catalog or the window cannot start.
pub fn run() -> Result<(), String> {
    let catalog = Catalog::embedded().map_err(|e| format!("catalog: {e}"))?;
    let texts = CatalogTexts::embedded().map_err(|e| format!("translations: {e}"))?;
    let (settings, warning) = match Settings::load_from(&Settings::path()) {
        Ok(s) => (s, None),
        Err(e) => (Settings::default(), Some(e)),
    };
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
        Box::new(move |_cc| {
            let mut app = App::new(catalog, texts, settings);
            if let Some(w) = warning {
                app.status = format!("Settings not read, the defaults are used: {w}");
            }
            Ok(Box::new(app))
        }),
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
        assert_eq!(
            with_extension(PathBuf::from("x")),
            PathBuf::from("x.wwprofile")
        );
        assert_eq!(
            with_extension(PathBuf::from("x.json")),
            PathBuf::from("x.json")
        );
    }

    #[test]
    fn app_starts_with_the_office_preset() {
        let catalog = Catalog::embedded().unwrap();
        let app = App::new(
            catalog,
            CatalogTexts::embedded().unwrap(),
            Settings::default(),
        );
        assert_eq!(app.docs.len(), 1);
        assert_eq!(app.docs[0].title(), "Office.wwprofile");
        assert!(!app.docs[0].dirty());
        assert_eq!(app.window_title(), "Office.wwprofile - WinWeaver");
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

    #[test]
    fn opening_a_file_adds_a_tab_and_remembers_it() {
        let catalog = Catalog::embedded().unwrap();
        let dir = std::env::temp_dir().join(format!("winweaver-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("strict.wwprofile");
        std::fs::write(&path, PRESETS[1].1).unwrap();
        let mut app = App::new(catalog, Vec::new(), Settings::default());
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.active, 1);
        assert_eq!(app.settings.recent, vec![path.clone()]);
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), 2, "the same file activates its tab");
        app.open_path(dir.join("missing.wwprofile"));
        assert!(app.status.starts_with("Open failed"), "{}", app.status);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
