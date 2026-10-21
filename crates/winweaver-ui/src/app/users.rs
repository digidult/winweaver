//! The users files: their tab, the master password, the accounts of this PC and of a profile.

use crate::*;

impl App {
    /// Unlocks the passwords of the users file in front, sets or changes their master password
    /// (`set`), or sets the password of the selected account (`password`) once unlocked.
    pub(crate) fn users_unlock(&mut self, set: bool, password: bool) {
        let Some(d) = self.active_users() else {
            return;
        };
        if password && d.unlocked() {
            d.start_password();
        } else if d.file.passwords.is_none() || (set && d.unlocked()) {
            d.ask.open(usersview::Purpose::Set);
        } else {
            d.ask.open(usersview::Purpose::Unlock);
        }
    }

    pub(crate) fn active_users(&mut self) -> Option<&mut usersview::UsersDoc> {
        match self.docs.get_mut(self.active) {
            Some(Doc::Users(d)) => Some(d),
            _ => None,
        }
    }

    /// A new users file in a tab of its own, empty or of the accounts of this PC.
    pub(crate) fn new_users_doc(&mut self, read: Vec<users::User>) {
        let mut file = UsersFile::new(tr("Users"));
        file.users = read;
        let mut doc = usersview::UsersDoc::new(file, None);
        doc.dirty = !doc.file.users.is_empty();
        doc.check();
        self.docs.push(Doc::Users(Box::new(doc)));
        self.active = self.docs.len() - 1;
    }

    /// Opens a users file in a tab of its own, or brings its tab to the front.
    pub(crate) fn open_users_path(&mut self, path: PathBuf) {
        let same = |d: &Doc| matches!(d, Doc::Users(u) if u.path.as_ref() == Some(&path));
        if let Some(i) = self.docs.iter().position(same) {
            self.active = i;
            return;
        }
        match UsersFile::load(&path) {
            Ok(file) => {
                let mut doc = usersview::UsersDoc::new(file, Some(path.clone()));
                doc.check();
                self.docs.push(Doc::Users(Box::new(doc)));
                self.active = self.docs.len() - 1;
                self.status = trf("Opened {path}", &[("path", &path.display())]);
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// Saves the users file in front, with its passwords encrypted again, at its place or at
    /// one the user picks.
    pub(crate) fn save_users_doc(&mut self, ask: bool) {
        let Some(Doc::Users(d)) = self.docs.get(self.active) else {
            return;
        };
        let path = match (&d.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = format!("{}.{}", file_stem(&d.file.name), users::EXTENSION);
                let dialog = file_dialog(tr("Users files"), users::EXTENSION)
                    .set_title(tr("Save the users file as"))
                    .set_file_name(name);
                match dialog.save_file() {
                    Some(p) if p.extension().is_some() => p,
                    Some(p) => p.with_extension(users::EXTENSION),
                    None => return,
                }
            }
        };
        let Some(Doc::Users(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        if let Err(e) = d.seal() {
            self.status = trf("Save failed: {e}", &[("e", &e)]);
            return;
        }
        match d.file.save(&path) {
            Ok(()) => {
                d.path = Some(path.clone());
                d.dirty = false;
                self.status = trf("Saved {path}", &[("path", &path.display())]);
                self.shown_file = Some((self.status.clone(), path.clone()));
                self.remember_recent(&path);
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// Asks before an operation of the users file in front on this PC, once its plan is sound.
    pub(crate) fn ask_users_run(&mut self, operation: users::Operation, all: bool) {
        if !winweaver_win::elevated() {
            self.status =
                tr("Run WinWeaver as administrator to change the accounts of this PC").to_owned();
            return;
        }
        let Some(d) = self.active_users() else {
            return;
        };
        let Some(passwords) = &d.passwords else {
            self.status = tr("Unlock the passwords first").to_owned();
            return;
        };
        let names = d.chosen(all);
        match users::Plan::new(operation, &d.file.users, &names, passwords) {
            Ok(plan) => {
                if plan.users.is_empty() {
                    self.status = tr("The file holds no accounts").to_owned();
                    return;
                }
                let text = d.question(operation, &names);
                self.ask(Question::new(
                    tr("Change this PC"),
                    text,
                    tr("Change this PC"),
                    UsersAct::Run(operation, names),
                ));
            }
            Err(issues) => {
                d.issues = issues;
                self.status = tr("The operation was not started: see the messages").to_owned();
            }
        }
    }

    /// Runs a confirmed operation on this PC and shows its outcomes in the tab.
    pub(crate) fn run_users(&mut self, operation: users::Operation, names: &[String]) {
        let Some(d) = self.active_users() else {
            return;
        };
        let Some(passwords) = &d.passwords else {
            return;
        };
        let plan = match users::Plan::new(operation, &d.file.users, names, passwords) {
            Ok(plan) => plan,
            Err(issues) => {
                d.issues = issues;
                return;
            }
        };
        let outcomes = winweaver_win::run_users_plan(&plan, passwords);
        let failed = outcomes.iter().filter(|o| o.status.failed()).count();
        let n = outcomes.len();
        d.outcomes = outcomes;
        self.status = trf(
            "This PC changed: {n} accounts, {failed} failed",
            &[("n", &n), ("failed", &failed)],
        );
    }

    /// Saves the stand-alone script of the users file in front next to the file it reads.
    pub(crate) fn save_users_script(&mut self) {
        let Some(Doc::Users(d)) = self.docs.get(self.active) else {
            return;
        };
        let Some(path) = d.path.clone() else {
            self.status =
                tr("Save the users file first: the script reads it next to itself").to_owned();
            return;
        };
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = match users::script(&file_name, winweaver_core::BUILD) {
            Ok(text) => text.replace("\r\n", "\n").replace('\n', "\r\n"),
            Err(e) => {
                self.status = trf("Script not made: {e}", &[("e", &e)]);
                return;
            }
        };
        let stem = file_stem(&d.file.name);
        let mut dialog = rfd::FileDialog::new()
            .set_title(tr("Save the script as"))
            .add_filter("PowerShell", &["ps1"])
            .set_file_name(format!("Create-{stem}.ps1"));
        if let Some(dir) = path.parent().filter(|p| p.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(target) = dialog.save_file() else {
            return;
        };
        match write_file(&target, text.as_bytes()) {
            Ok(()) => {
                self.status = trf(
                    "Saved {path}; keep {file} next to it and run it as administrator",
                    &[("path", &target.display()), ("file", &file_name)],
                );
                self.shown_file = Some((self.status.clone(), target));
            }
            Err(e) => self.status = trf("Save failed: {e}", &[("e", &e)]),
        }
    }

    /// A users file of the accounts of the profile in front, in a tab of its own; the passwords
    /// of the profile wait for a master password before the first save.
    pub(crate) fn users_from_profile(&mut self) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let (users, passwords) = users::from_accounts(&doc.profile.accounts);
        let name = trf("{name} users", &[("name", &doc.profile.name)]);
        let mut file = UsersFile::new(&name);
        file.users = users;
        let mut tab = usersview::UsersDoc::new(file, None);
        tab.dirty = true;
        if !passwords.is_empty() {
            tab.passwords = Some(passwords);
            tab.ask.open(usersview::Purpose::Set);
        }
        tab.check();
        self.docs.push(Doc::Users(Box::new(tab)));
        self.active = self.docs.len() - 1;
    }

    /// Loads the accounts of a users file into the profile in front, asking the master password
    /// for their passwords when the file holds some.
    pub(crate) fn users_into_profile(&mut self) {
        if self.active_profile().is_none() {
            return;
        }
        let dialog = file_dialog(tr("Users files"), users::EXTENSION)
            .set_title(tr("Accounts from a users file"));
        let Some(path) = dialog.pick_file() else {
            return;
        };
        match UsersFile::load(&path) {
            Ok(file) if file.passwords.is_some() => {
                let mut ask = usersview::MasterAsk::default();
                ask.open(usersview::Purpose::Unlock);
                self.users_import = Some(usersview::UsersImport { file, ask });
            }
            Ok(file) => {
                let n = file.users.len();
                self.apply(vec![Act::Profile(ProfileAct::SetAccounts(
                    file.accounts(None),
                ))]);
                self.status = trf("{n} accounts loaded without passwords", &[("n", &n)]);
            }
            Err(e) => self.status = trf("Open failed: {e}", &[("e", &e)]),
        }
    }

    /// The window of the master password of a users file loaded into a profile.
    pub(crate) fn users_import_view(&mut self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(import) = &mut self.users_import else {
            return;
        };
        match usersview::import_window(ctx, import) {
            usersview::ImportDone::Open => {}
            usersview::ImportDone::Cancel => self.users_import = None,
            usersview::ImportDone::Without => {
                acts.push(Act::Profile(ProfileAct::SetAccounts(
                    import.file.accounts(None),
                )));
                self.users_import = None;
            }
            usersview::ImportDone::With(passwords) => {
                acts.push(Act::Profile(ProfileAct::SetAccounts(
                    import.file.accounts(Some(&passwords)),
                )));
                self.users_import = None;
            }
        }
    }

    /// Runs an act of the acts of a users file.
    pub(crate) fn apply_users(&mut self, act: UsersAct) {
        match act {
            UsersAct::New => self.new_users_doc(Vec::new()),
            UsersAct::OfThisPc => match winweaver_win::users_of_this_pc() {
                Ok(read) => self.new_users_doc(read),
                Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
            },
            UsersAct::Add => {
                if let Some(d) = self.active_users() {
                    let n = d.file.users.len() + 1;
                    d.add(users::User {
                        name: format!("User{n}"),
                        ..users::User::default()
                    });
                }
            }
            UsersAct::Duplicate => {
                if let Some(d) = self.active_users() {
                    d.duplicate();
                }
            }
            UsersAct::Remove(i) => {
                if let Some(d) = self.active_users() {
                    d.remove(i);
                }
            }
            UsersAct::Move(down) => {
                if let Some(d) = self.active_users() {
                    d.move_selected(down);
                }
            }
            UsersAct::Unlock => self.users_unlock(false, false),
            UsersAct::SetMaster => self.users_unlock(true, false),
            UsersAct::SetPassword => self.users_unlock(false, true),
            UsersAct::Lock => {
                if let Some(d) = self.active_users() {
                    d.lock();
                }
            }
            UsersAct::Check => {
                if let Some(d) = self.active_users() {
                    d.check();
                    let errors = d.issues.iter().filter(|i| i.error).count();
                    self.status = trf(
                        "Checked {n} accounts: {errors} errors",
                        &[("n", &d.file.users.len()), ("errors", &errors)],
                    );
                }
            }
            UsersAct::ReadPc => match winweaver_win::users_of_this_pc() {
                Ok(read) => {
                    if let Some(d) = self.active_users() {
                        let n = d.merge(read);
                        self.status = trf("{n} accounts of this PC added", &[("n", &n)]);
                    }
                }
                Err(e) => self.status = trf("This PC was not read: {e}", &[("e", &e)]),
            },
            UsersAct::AskRun(operation, all) => self.ask_users_run(operation, all),
            UsersAct::Run(operation, names) => self.run_users(operation, &names),
            UsersAct::Script => self.save_users_script(),
            UsersAct::IntoProfile => self.users_into_profile(),
            UsersAct::FromProfile => self.users_from_profile(),
        }
    }
}
