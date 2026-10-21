//! The tab of a users file (`.wwusers`): the accounts in a list, the selected one in a form, the
//! master password that unlocks the passwords of the file, and the outcomes of an operation on
//! this PC. The passwords live in memory only while the file is unlocked.

use std::path::PathBuf;

use eframe::egui;
use winweaver_core::users::{self, Issue, Operation, Outcome, State, User, UsersFile};
use winweaver_core::userstore::{self, Passwords};
use zeroize::Zeroizing;

use crate::i18n::{tr, trf};
use crate::tree::Scroll;
use crate::{Act, Question, Search, UsersAct, icons, list, matches_words, style, widgets};

/// Why the master password is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Open the passwords the file holds.
    Unlock,
    /// Set the master password of a file without passwords, or change it.
    Set,
}

/// The window that asks the master password.
#[derive(Default)]
pub struct MasterAsk {
    pub purpose: Option<Purpose>,
    pub text: String,
    pub again: String,
    pub error: String,
}

impl MasterAsk {
    pub fn open(&mut self, purpose: Purpose) {
        *self = Self::default();
        self.purpose = Some(purpose);
    }
}

/// The window that sets the password of an account.
pub struct PasswordEdit {
    pub name: String,
    pub text: String,
    pub again: String,
    pub error: String,
}

pub struct UsersDoc {
    pub path: Option<PathBuf>,
    pub file: UsersFile,
    pub dirty: bool,
    pub selected: Option<usize>,
    /// The passwords while the file is unlocked.
    pub passwords: Option<Passwords>,
    /// The master password while the file is unlocked, for the save.
    master: Option<Zeroizing<Vec<u8>>>,
    pub search: Search,
    pub follow: bool,
    pub issues: Vec<Issue>,
    pub outcomes: Vec<Outcome>,
    pub ask: MasterAsk,
    pub password_edit: Option<PasswordEdit>,
    /// The password of the selected account is shown instead of dots.
    pub show_password: bool,
    pub error: String,
}

impl UsersDoc {
    pub fn new(file: UsersFile, path: Option<PathBuf>) -> Self {
        Self {
            path,
            file,
            dirty: false,
            selected: None,
            passwords: None,
            master: None,
            search: Search::default(),
            follow: false,
            issues: Vec::new(),
            outcomes: Vec::new(),
            ask: MasterAsk::default(),
            password_edit: None,
            show_password: false,
            error: String::new(),
        }
    }

    /// The question before the removal of the selected account from the file.
    pub fn remove_question(&self) -> Option<Question> {
        let i = self.selected?;
        let name = self.selected_user()?.name.clone();
        Some(Question::new(
            tr("Remove the account from the file"),
            trf(
                "Remove {name} from the file? An account of a PC stays.",
                &[("name", &name)],
            ),
            tr("Remove"),
            UsersAct::Remove(i),
        ))
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.file.name.clone()),
            None => self.file.name.clone(),
        }
    }

    pub fn unlocked(&self) -> bool {
        self.passwords.is_some()
    }

    pub fn selected_user(&self) -> Option<&User> {
        self.selected.and_then(|i| self.file.users.get(i))
    }

    /// Opens the passwords with a master password, or starts a new set of them.
    fn enter_master(&mut self, purpose: Purpose, master: &str) -> Result<(), String> {
        let bytes = Zeroizing::new(master.as_bytes().to_vec());
        let passwords = match (purpose, &self.file.passwords) {
            (Purpose::Unlock, Some(container)) => userstore::decrypt(container, &bytes)?,
            (Purpose::Unlock, None) => return Err(tr("The file holds no passwords").to_owned()),
            (Purpose::Set, _) => self.passwords.take().unwrap_or_default(),
        };
        self.passwords = Some(passwords);
        self.master = Some(bytes);
        if purpose == Purpose::Set {
            self.dirty = true;
        }
        self.check();
        Ok(())
    }

    /// Drops the passwords and the master password.
    pub fn lock(&mut self) {
        self.passwords = None;
        self.master = None;
        self.password_edit = None;
        self.show_password = false;
        self.check();
    }

    /// Encrypts the passwords into the file before a save; `false` when the file is unlocked
    /// without a master password, which cannot happen.
    pub fn seal(&mut self) -> Result<(), String> {
        if self.passwords.is_some() && self.master.is_none() {
            return Err(tr("Set a master password first").to_owned());
        }
        if let (Some(passwords), Some(master)) = (&self.passwords, &self.master) {
            let iterations = self
                .file
                .passwords
                .as_ref()
                .map_or(userstore::DEFAULT_ITERATIONS, |c| c.iterations)
                .max(userstore::MIN_ITERATIONS);
            self.file.passwords = Some(userstore::encrypt(passwords, master, iterations)?);
        }
        Ok(())
    }

    /// The findings about the list and, when unlocked, about the passwords.
    pub fn check(&mut self) {
        self.issues = users::check(&self.file.users);
        if let Some(passwords) = &self.passwords {
            self.issues
                .extend(users::check_passwords(&self.file.users, passwords));
        }
    }

    pub fn add(&mut self, user: User) {
        self.file.users.push(user);
        self.selected = Some(self.file.users.len() - 1);
        self.dirty = true;
        self.follow = true;
        self.check();
    }

    /// Adds the accounts of this PC that the list lacks; how many came.
    pub fn merge(&mut self, read: Vec<User>) -> usize {
        let mut added = 0;
        for user in read {
            if self.file.find(&user.name).is_none() {
                self.file.users.push(user);
                added += 1;
            }
        }
        if added > 0 {
            self.dirty = true;
            self.check();
        }
        added
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.file.users.len() {
            return;
        }
        let gone = self.file.users.remove(index);
        if let Some(p) = &mut self.passwords {
            p.remove(&gone.name);
        }
        self.selected = match self.selected {
            Some(_) if self.file.users.is_empty() => None,
            Some(i) => Some(i.min(self.file.users.len() - 1)),
            None => None,
        };
        self.dirty = true;
        self.check();
    }

    pub fn duplicate(&mut self) {
        if let Some(user) = self.selected_user().cloned() {
            let mut copy = user;
            let base = copy.name.clone();
            let mut n = 2;
            while self.file.find(&format!("{base}{n}")).is_some() {
                n += 1;
            }
            copy.name = format!("{base}{n}");
            self.add(copy);
        }
    }

    pub fn move_selected(&mut self, down: bool) {
        let Some(i) = self.selected else {
            return;
        };
        let j = if down { i + 1 } else { i.wrapping_sub(1) };
        if j < self.file.users.len() {
            self.file.users.swap(i, j);
            self.selected = Some(j);
            self.dirty = true;
            self.follow = true;
        }
    }

    /// Starts the window that sets the password of the selected account; the file must be
    /// unlocked.
    pub fn start_password(&mut self) {
        if let Some(user) = self.selected_user() {
            self.password_edit = Some(PasswordEdit {
                name: user.name.clone(),
                text: String::new(),
                again: String::new(),
                error: String::new(),
            });
        }
    }

    /// The names of the accounts the operation takes: the selected one, or all of them.
    pub fn chosen(&self, all: bool) -> Vec<String> {
        if all {
            Vec::new()
        } else {
            self.selected_user()
                .map(|u| vec![u.name.clone()])
                .unwrap_or_default()
        }
    }

    /// The text of the question before an operation on this PC.
    pub fn question(&self, operation: Operation, names: &[String]) -> String {
        let count = if names.is_empty() {
            self.file.users.len()
        } else {
            names.len()
        };
        let what = match operation {
            Operation::Passwords => tr("Set the passwords of {n} accounts of this PC?"),
            Operation::Create { .. } => tr("Create or change {n} accounts of this PC?"),
        };
        let mut text = what.replace("{n}", &count.to_string());
        let touching: Vec<&str> = self
            .file
            .users
            .iter()
            .filter(|u| names.is_empty() || names.iter().any(|n| n.eq_ignore_ascii_case(&u.name)))
            .filter(|u| u.touches_existing())
            .map(|u| u.name.as_str())
            .collect();
        if !touching.is_empty() && matches!(operation, Operation::Create { .. }) {
            text.push('\n');
            text.push_str(&trf(
                "Accounts that exist are changed by their orders: {names}",
                &[("names", &touching.join(", "))],
            ));
        }
        text
    }
}

fn state_text(state: State) -> &'static str {
    match state {
        State::Keep => tr("keep"),
        State::Enabled => tr("enabled"),
        State::Disabled => tr("disabled"),
    }
}

/// The list of the accounts, with the search of the tab applied.
fn list_panel(ui: &mut egui::Ui, doc: &mut UsersDoc, acts: &mut Vec<Act>) {
    let words = doc.search.words();
    let columns = [
        list::Column::new(tr("Name"), 140.0),
        list::Column::new(tr("Group"), 110.0),
        list::Column::new(tr("State"), 80.0),
        list::Column::new(tr("Password"), 80.0),
        list::Column::new(tr("Profile"), 160.0),
    ];
    let rows: Vec<list::Row<usize>> = doc
        .file
        .users
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            words.is_empty() || {
                let text = format!("{} {} {}", u.name, u.full_name, u.profile_path);
                matches_words(&text, &words)
            }
        })
        .map(|(i, u)| {
            let bad = doc.issues.iter().any(|x| x.error && x.user == u.name);
            let password = match &doc.passwords {
                Some(p) if p.has(&u.name) => list::Cell::new(tr("yes")),
                Some(_) => list::Cell::new(tr("no")).warn(),
                None => list::Cell::new(tr("locked")).muted(),
            };
            let mut name = list::Cell::new(u.name.clone()).strong();
            if bad {
                name = name.warn();
            }
            list::Row {
                key: i,
                cells: vec![
                    name,
                    list::Cell::new(u.group()),
                    list::Cell::new(state_text(u.state)),
                    password,
                    list::Cell::new(u.profile_path.clone()).muted(),
                ],
                button: None,
            }
        })
        .collect();
    let selected = doc
        .selected
        .and_then(|i| rows.iter().position(|r| r.key == i));
    let scroll = Scroll::wanted(false, doc.follow);
    doc.follow = false;
    let out = list::show(
        ui,
        egui::Id::new("users-list"),
        tr("Accounts"),
        &columns,
        &rows,
        selected,
        scroll,
    );
    for event in out.events {
        match event {
            list::Event::Select(i) | list::Event::Open(i) => doc.selected = Some(i),
            list::Event::Delete(_) => acts.extend(doc.remove_question().map(Act::from)),
            list::Event::Button(_) => {}
        }
    }
    if out.moved {
        doc.follow = true;
    }
}

/// The form of the selected account; a change marks the file dirty.
fn user_form(ui: &mut egui::Ui, doc: &mut UsersDoc) {
    let Some(i) = doc.selected.filter(|i| *i < doc.file.users.len()) else {
        ui.weak(tr("Select an account, or add one."));
        return;
    };
    let before = doc.file.users[i].clone();
    let mut u = before.clone();
    egui::Grid::new("user-form")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("Name"));
            ui.add(egui::TextEdit::singleline(&mut u.name).desired_width(200.0));
            ui.end_row();
            ui.label(tr("Full name"));
            ui.add(egui::TextEdit::singleline(&mut u.full_name).desired_width(260.0));
            ui.end_row();
            ui.label(tr("Description"));
            ui.add(egui::TextEdit::singleline(&mut u.description).desired_width(320.0));
            ui.end_row();
            ui.label(tr("Group"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut u.admin, false, "Users");
                ui.radio_value(&mut u.admin, true, "Administrators");
            });
            ui.end_row();
            ui.label(tr("Profile folder"));
            ui.add(
                egui::TextEdit::singleline(&mut u.profile_path)
                    .hint_text(tr("Empty: the one Windows chooses"))
                    .desired_width(320.0),
            );
            ui.end_row();
            ui.label("");
            ui.checkbox(
                &mut u.password_never_expires,
                tr("The password never expires"),
            );
            ui.end_row();
        });
    ui.add_space(8.0);
    style::section_title(ui, tr("When the account already exists on the PC"));
    ui.weak(tr(
        "Without these orders an account that exists is left as it is.",
    ));
    ui.checkbox(&mut u.apply_password, tr("Set the password of the file"));
    ui.horizontal(|ui| {
        ui.label(tr("State"));
        for state in State::ALL {
            ui.radio_value(&mut u.state, state, state_text(state));
        }
    });
    ui.checkbox(
        &mut u.force_group,
        tr("Exactly this group: remove the other membership"),
    );
    if doc.unlocked() {
        password_row(ui, doc, &before.name);
    }
    if u != before {
        if u.name != before.name
            && let Some(p) = &mut doc.passwords
        {
            p.rename(&before.name, &u.name);
        }
        doc.file.users[i] = u;
        doc.dirty = true;
        doc.check();
    }
}

/// The password of the selected account while the file is unlocked: dots, or the text with
/// Show, and a copy to the clipboard.
fn password_row(ui: &mut egui::Ui, doc: &mut UsersDoc, name: &str) {
    let password: Option<Zeroizing<String>> = doc
        .passwords
        .as_ref()
        .and_then(|p| p.get(name))
        .map(|p| Zeroizing::new(p.to_owned()));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Password"));
        let Some(password) = &password else {
            ui.weak(tr("none"));
            return;
        };
        if doc.show_password {
            ui.monospace(password.as_str());
        } else {
            ui.monospace(icons::BULLET.repeat(8));
        }
        let eye = if doc.show_password {
            tr("Hide")
        } else {
            tr("Show")
        };
        if ui.small_button(eye).clicked() {
            doc.show_password = !doc.show_password;
        }
        if ui.small_button(tr("Copy")).clicked() {
            ui.ctx().copy_text(password.to_string());
        }
    });
    if password.is_some() {
        ui.weak(tr("The clipboard history of Windows keeps what is copied."));
    }
}

/// The findings and the outcomes of the last operation.
fn messages(ui: &mut egui::Ui, doc: &UsersDoc) {
    let warn = ui.visuals().warn_fg_color;
    if !doc.error.is_empty() {
        ui.colored_label(warn, &doc.error);
    }
    for issue in &doc.issues {
        let text = if issue.user.is_empty() {
            issue.text.clone()
        } else {
            format!("{}: {}", issue.user, issue.text)
        };
        if issue.error {
            ui.colored_label(warn, text);
        } else {
            ui.weak(text);
        }
    }
    if !doc.outcomes.is_empty() {
        ui.add_space(6.0);
        style::section_title(ui, tr("The last operation on this PC"));
        for o in &doc.outcomes {
            let line = if o.message.is_empty() {
                format!("{}: {}", o.name, o.status.as_str())
            } else {
                format!("{}: {} ({})", o.name, o.status.as_str(), o.message)
            };
            if o.status.failed() {
                ui.colored_label(warn, line);
            } else {
                ui.label(line);
            }
        }
    }
}

/// The window of the master password; `true` once it closed.
fn master_window(ui: &mut egui::Ui, doc: &mut UsersDoc) {
    let Some(purpose) = doc.ask.purpose else {
        return;
    };
    let title = match purpose {
        Purpose::Unlock => tr("Unlock the passwords"),
        Purpose::Set => tr("Set the master password"),
    };
    let mut done = None;
    widgets::Modal::new(title).show(ui.ctx(), |ui| {
        ui.label(tr("Master password"));
        let field = egui::TextEdit::singleline(&mut doc.ask.text)
            .password(true)
            .desired_width(240.0);
        let response = ui.add(field);
        let mut enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if purpose == Purpose::Set {
            ui.label(tr("Once more"));
            let again = egui::TextEdit::singleline(&mut doc.ask.again)
                .password(true)
                .desired_width(240.0);
            let response = ui.add(again);
            enter |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.weak(tr(
                "Without it the passwords cannot be read again: there is no way to recover them.",
            ));
        }
        if !doc.ask.error.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, &doc.ask.error);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("OK")).clicked() || enter {
                done = Some(true);
            }
            if ui.button(tr("Cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
        });
    });
    match done {
        Some(false) => doc.ask = MasterAsk::default(),
        Some(true) => {
            if purpose == Purpose::Set && doc.ask.text != doc.ask.again {
                doc.ask.error = tr("The two entries differ").to_owned();
            } else if doc.ask.text.is_empty() {
                doc.ask.error = tr("The master password is empty").to_owned();
            } else {
                let text = std::mem::take(&mut doc.ask.text);
                match doc.enter_master(purpose, &text) {
                    Ok(()) => doc.ask = MasterAsk::default(),
                    Err(e) => doc.ask.error = e,
                }
                doc.ask.again.clear();
            }
        }
        None => {}
    }
}

/// The window that sets the password of an account.
fn password_window(ui: &mut egui::Ui, doc: &mut UsersDoc) {
    let Some(edit) = &mut doc.password_edit else {
        return;
    };
    let mut done = None;
    let title = trf("The password of {name}", &[("name", &edit.name)]);
    widgets::Modal::new(title).show(ui.ctx(), |ui| {
        ui.label(tr("Password"));
        let field = egui::TextEdit::singleline(&mut edit.text)
            .password(true)
            .desired_width(240.0);
        ui.add(field);
        ui.label(tr("Once more"));
        let again = egui::TextEdit::singleline(&mut edit.again)
            .password(true)
            .desired_width(240.0);
        let response = ui.add(again);
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !edit.error.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, &edit.error);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("OK")).clicked() || enter {
                done = Some(true);
            }
            if ui.button(tr("Cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
        });
    });
    match done {
        Some(false) => doc.password_edit = None,
        Some(true) => {
            let Some(edit) = doc.password_edit.as_mut() else {
                return;
            };
            if edit.text != edit.again {
                edit.error = tr("The two entries differ").to_owned();
                return;
            }
            if edit.text.is_empty() {
                edit.error = tr("The password is empty").to_owned();
                return;
            }
            let name = edit.name.clone();
            let text = std::mem::take(&mut edit.text);
            if let Some(p) = &mut doc.passwords {
                p.set(&name, &text);
            }
            doc.password_edit = None;
            doc.dirty = true;
            doc.check();
        }
        None => {}
    }
}

/// The accounts of a users file on their way into a profile: the file and the master password
/// asked for its passwords.
pub struct UsersImport {
    pub file: UsersFile,
    pub ask: MasterAsk,
}

pub enum ImportDone {
    Open,
    Cancel,
    Without,
    With(Passwords),
}

/// The window that asks the master password of a users file loaded into a profile; the accounts
/// may also come without their passwords.
pub fn import_window(ctx: &egui::Context, import: &mut UsersImport) -> ImportDone {
    let mut done = ImportDone::Open;
    let mut unlock = false;
    widgets::Modal::new(tr("Accounts from a users file")).show(ctx, |ui| {
        let n = import.file.users.len();
        ui.label(trf(
            "{n} accounts of {name}",
            &[("n", &n), ("name", &import.file.name)],
        ));
        ui.label(tr("Master password, for the passwords of the accounts"));
        let field = egui::TextEdit::singleline(&mut import.ask.text)
            .password(true)
            .desired_width(240.0);
        let response = ui.add(field);
        unlock = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !import.ask.error.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, &import.ask.error);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("With the passwords")).clicked() {
                unlock = true;
            }
            if ui.button(tr("Without the passwords")).clicked() {
                done = ImportDone::Without;
            }
            if ui.button(tr("Cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = ImportDone::Cancel;
            }
        });
    });
    if unlock && let Some(container) = &import.file.passwords {
        let master = Zeroizing::new(std::mem::take(&mut import.ask.text).into_bytes());
        match userstore::decrypt(container, &master) {
            Ok(passwords) => done = ImportDone::With(passwords),
            Err(e) => import.ask.error = e,
        }
    }
    done
}

pub fn users_view(ui: &mut egui::Ui, doc: &mut UsersDoc, acts: &mut Vec<Act>) {
    egui::Panel::left("users-side")
        .resizable(true)
        .default_size(440.0)
        .show(ui, |ui| {
            list_panel(ui, doc, acts);
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Name of the file"));
                let name = egui::TextEdit::singleline(&mut doc.file.name).desired_width(200.0);
                if ui.add(name).changed() {
                    doc.dirty = true;
                }
                let state = if doc.unlocked() {
                    tr("Passwords: unlocked")
                } else if doc.file.passwords.is_some() {
                    tr("Passwords: locked")
                } else {
                    tr("Passwords: none, set a master password")
                };
                let _ = style::chip(ui, state, doc.unlocked());
            });
            ui.add_space(8.0);
            user_form(ui, doc);
            ui.add_space(8.0);
            messages(ui, doc);
        });
    });
    master_window(ui, doc);
    password_window(ui, doc);
}
