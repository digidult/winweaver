//! The users files: their tab, the master password, the accounts of this PC and of a profile.

use crate::*;

impl WinWeaver {
    /// Unlocks the passwords of the users file in front, sets or changes their master password
    /// (`set`), or sets the password of the selected account (`password`) once unlocked.
    pub(crate) fn users_unlock(&mut self, shell: &mut Shell<WinWeaver>, set: bool, password: bool) {
        let Some(d) = self.active_users(shell) else {
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

    pub(crate) fn active_users(&mut self, shell: &mut Shell<WinWeaver>) -> Option<&mut usersview::UsersDoc> {
        match shell.docs.get_mut(shell.active) {
            Some(Doc::Users(d)) => Some(d),
            _ => None,
        }
    }

    /// A new users file in a tab of its own, empty or of the accounts of this PC.
    pub(crate) fn new_users_doc(&mut self, shell: &mut Shell<WinWeaver>, read: Vec<users::User>) {
        let mut file = UsersFile::new(tr("Users"));
        file.users = read;
        let mut doc = usersview::UsersDoc::new(file, None);
        doc.dirty = !doc.file.users.is_empty();
        doc.check();
        shell.open_tab(Doc::Users(Box::new(doc)));
    }

    /// Opens a users file in a tab of its own, or brings its tab to the front.
    pub(crate) fn open_users_path(&mut self, shell: &mut Shell<WinWeaver>, path: PathBuf) {
        shell.open_file(
            &path,
            |d| matches!(d, Doc::Users(u) if u.path.as_ref() == Some(&path)),
            UsersFile::load,
            |file, path| {
                let mut doc = usersview::UsersDoc::new(file, Some(path));
                doc.check();
                Doc::Users(Box::new(doc))
            },
        );
    }

    /// Saves the users file in front, with its passwords encrypted again, at its place or at
    /// one the user picks.
    pub(crate) fn save_users_doc(&mut self, shell: &mut Shell<WinWeaver>, ask: bool) {
        let Some(Doc::Users(d)) = shell.docs.get(shell.active) else {
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
        let Some(Doc::Users(d)) = shell.docs.get_mut(shell.active) else {
            return;
        };
        if let Err(e) = d.seal() {
            shell.save_failed(e);
            return;
        }
        match d.file.save(&path) {
            Ok(()) => {
                d.path = Some(path.clone());
                d.dirty = false;
                shell.saved(&path);
                shell.remember_recent(&path);
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// Asks before an operation of the users file in front on this PC, once its plan is sound.
    pub(crate) fn ask_users_run(&mut self, shell: &mut Shell<WinWeaver>, operation: users::Operation, all: bool) {
        if !winweaver_win::elevated() {
            shell.status =
                tr("Run WinWeaver as administrator to change the accounts of this PC").to_owned();
            return;
        }
        let Some(d) = self.active_users(shell) else {
            return;
        };
        let Some(passwords) = &d.passwords else {
            shell.status = tr("Unlock the passwords first").to_owned();
            return;
        };
        let names = d.chosen(all);
        match users::Plan::new(operation, &d.file.users, &names, passwords) {
            Ok(plan) => {
                if plan.users.is_empty() {
                    shell.status = tr("The file holds no accounts").to_owned();
                    return;
                }
                let text = d.question(operation, &names);
                shell.ask(Question::new(
                    tr("Change this PC"),
                    text,
                    tr("Change this PC"),
                    UsersAct::Run(operation, names),
                ));
            }
            Err(issues) => {
                d.issues = issues;
                shell.status = tr("The operation was not started: see the messages").to_owned();
            }
        }
    }

    /// Runs a confirmed operation on this PC and shows its outcomes in the tab.
    pub(crate) fn run_users(&mut self, shell: &mut Shell<WinWeaver>, operation: users::Operation, names: &[String]) {
        let Some(d) = self.active_users(shell) else {
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
        shell.status = trf(
            "This PC changed: {n} accounts, {failed} failed",
            &[("n", &n), ("failed", &failed)],
        );
    }

    /// Saves the stand-alone script of the users file in front next to the file it reads.
    pub(crate) fn save_users_script(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Users(d)) = shell.docs.get(shell.active) else {
            return;
        };
        let Some(path) = d.path.clone() else {
            shell.status =
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
                shell.status = trf("Script not made: {e}", &[("e", &e)]);
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
                shell.status = trf(
                    "Saved {path}; keep {file} next to it and run it as administrator",
                    &[("path", &target.display()), ("file", &file_name)],
                );
                shell.shown_file = Some((shell.status.clone(), target));
            }
            Err(e) => shell.save_failed(e),
        }
    }

    /// A users file of the accounts of the profile in front, in a tab of its own; the passwords
    /// of the profile wait for a master password before the first save.
    pub(crate) fn users_from_profile(&mut self, shell: &mut Shell<WinWeaver>) {
        let Some(Doc::Profile(doc)) = shell.docs.get(shell.active) else {
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
        shell.open_tab(Doc::Users(Box::new(tab)));
    }

    /// Loads the accounts of a users file into the profile in front, asking the master password
    /// for their passwords when the file holds some.
    pub(crate) fn users_into_profile(&mut self, shell: &mut Shell<WinWeaver>) {
        if self.active_profile(shell).is_none() {
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
                self.apply(shell, Act::Profile(ProfileAct::SetAccounts(
                    file.accounts(None),
                )));
                shell.status = trf("{n} accounts loaded without passwords", &[("n", &n)]);
            }
            Err(e) => shell.open_failed(e),
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
    pub(crate) fn apply_users(&mut self, shell: &mut Shell<WinWeaver>, act: UsersAct) {
        match act {
            UsersAct::New => self.new_users_doc(shell, Vec::new()),
            UsersAct::OfThisPc => match winweaver_win::users_of_this_pc() {
                Ok(read) => self.new_users_doc(shell, read),
                Err(e) => shell.status = trf("This PC was not read: {e}", &[("e", &e)]),
            },
            UsersAct::Add => {
                if let Some(d) = self.active_users(shell) {
                    let n = d.file.users.len() + 1;
                    d.add(users::User {
                        name: format!("User{n}"),
                        ..users::User::default()
                    });
                }
            }
            UsersAct::Duplicate => {
                if let Some(d) = self.active_users(shell) {
                    d.duplicate();
                }
            }
            UsersAct::Remove(i) => {
                if let Some(d) = self.active_users(shell) {
                    d.remove(i);
                }
            }
            UsersAct::Move(down) => {
                if let Some(d) = self.active_users(shell) {
                    d.move_selected(down);
                }
            }
            UsersAct::Unlock => self.users_unlock(shell, false, false),
            UsersAct::SetMaster => self.users_unlock(shell, true, false),
            UsersAct::SetPassword => self.users_unlock(shell, false, true),
            UsersAct::Lock => {
                if let Some(d) = self.active_users(shell) {
                    d.lock();
                }
            }
            UsersAct::Check => {
                if let Some(d) = self.active_users(shell) {
                    d.check();
                    let errors = d.issues.iter().filter(|i| i.error).count();
                    shell.status = trf(
                        "Checked {n} accounts: {errors} errors",
                        &[("n", &d.file.users.len()), ("errors", &errors)],
                    );
                }
            }
            UsersAct::ReadPc => match winweaver_win::users_of_this_pc() {
                Ok(read) => {
                    if let Some(d) = self.active_users(shell) {
                        let n = d.merge(read);
                        shell.status = trf("{n} accounts of this PC added", &[("n", &n)]);
                    }
                }
                Err(e) => shell.status = trf("This PC was not read: {e}", &[("e", &e)]),
            },
            UsersAct::AskRun(operation, all) => self.ask_users_run(shell, operation, all),
            UsersAct::Run(operation, names) => self.run_users(shell, operation, &names),
            UsersAct::Script => self.save_users_script(shell),
            UsersAct::IntoProfile => self.users_into_profile(shell),
            UsersAct::FromProfile => self.users_from_profile(shell),
        }
    }
}
