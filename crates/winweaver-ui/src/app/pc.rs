//! This PC: the read in its thread, the checks, the plans, the elevated run, the scripts and the reports.

use crate::*;

impl App {
    /// Reads this PC: every rule into a new profile tab, or the selection into the open profile.
    pub(crate) fn read_pc(&mut self, only_selection: bool) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status =
                tr("Read this PC: open a profile first; its rules and forms are the base")
                    .to_owned();
            return;
        };
        let ids = if only_selection {
            self.selection_ids()
        } else {
            self.catalog
                .rules()
                .iter()
                .filter(|r| !r.id.starts_with("admx."))
                .map(|r| r.id.clone())
                .collect()
        };
        let base = doc.profile.clone();
        let plan = thispc::plan(&self.catalog, &base, &ids);
        let job = if only_selection {
            PcJob::Selection {
                doc: doc.draft_key.clone(),
                base,
            }
        } else {
            PcJob::NewProfile { base }
        };
        self.start_pc_read(job, plan, !only_selection);
    }

    /// Compares the selected rules of the profile with this PC.
    pub(crate) fn check_on_pc(&mut self) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let plan = thispc::plan(&self.catalog, &doc.profile, &ids);
        let job = PcJob::Check {
            doc: doc.draft_key.clone(),
        };
        self.start_pc_read(job, plan, false);
    }

    /// Starts a read of this PC in a thread of its own, one at a time; `poll_pc_read` takes its
    /// report.
    pub(crate) fn start_pc_read(&mut self, job: PcJob, plan: thispc::Plan, read_system: bool) {
        if self.pc_read.is_some() {
            self.status = tr("This PC is being read; wait until the read ends").to_owned();
            return;
        }
        let Some(machine) = self.pc.clone() else {
            self.status = tr("This PC can be read on Windows only").to_owned();
            return;
        };
        let checks = plan.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name("this-pc".to_owned())
            .spawn(move || {
                let time = clock::now_utc_iso();
                let _ = tx.send(thispc::run(&*machine, &checks, read_system, &time));
            });
        if let Err(e) = started {
            self.status = trf("This PC could not be read: {e}", &[("e", &e)]);
            return;
        }
        self.pc_read = Some(PcRead {
            job,
            plan,
            report: rx,
        });
        self.status = tr("This PC is being read").to_owned();
    }

    /// Takes the report of a finished read of this PC.
    pub(crate) fn poll_pc_read(&mut self, ctx: &egui::Context) {
        let Some(read) = &self.pc_read else {
            return;
        };
        match read.report.try_recv() {
            Ok(report) => {
                if let Some(read) = self.pc_read.take() {
                    self.finish_pc_read(read.job, &read.plan, &report);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pc_read = None;
                self.status = tr("The read of this PC failed; the log names the error").to_owned();
            }
        }
    }

    /// Puts the report of a read of this PC where its job wants it.
    pub(crate) fn finish_pc_read(&mut self, job: PcJob, plan: &thispc::Plan, report: &Report) {
        let readings = thispc::judge(&self.catalog, plan, &report.results);
        match job {
            PcJob::Check { doc } => self.finish_check(&doc, plan, report, &readings),
            PcJob::NewProfile { base } => self.finish_capture(None, &base, plan, report, &readings),
            PcJob::Selection { doc, base } => {
                self.finish_capture(Some(&doc), &base, plan, report, &readings)
            }
        }
    }

    /// The capture of a read: a new profile tab, or the profile of the tab named by `target`.
    pub(crate) fn finish_capture(
        &mut self,
        target: Option<&str>,
        base: &Profile,
        plan: &thispc::Plan,
        report: &Report,
        readings: &[thispc::Reading],
    ) {
        let (mut profile, summary) =
            thispc::capture(&self.catalog, base, readings, report.system.as_ref());
        let mut issues = capture_issues(&summary);
        if !report.admin && plan.admin_checks() > 0 {
            issues.push(admin_issue());
        }
        if !plan.excluded.is_empty() {
            issues.push(Issue {
                severity: Severity::Info,
                target: Target::Profile,
                text: format!(
                    "This PC: {} rules act only during installation or at a first sign-in and were not read",
                    plan.excluded.len()
                ),
            });
        }
        let catalog = &self.catalog;
        if let Some(key) = target {
            let Some(doc) = profile_doc(&mut self.docs, key) else {
                self.status = tr("Read this PC: the tab of the profile was closed").to_owned();
                return;
            };
            doc.profile = profile;
            doc.refresh(catalog);
            doc.issues = issues;
            self.status = format!("Read the selection on this PC: {}", summary.line());
        } else {
            let computer = if report.computer.is_empty() {
                "This PC".to_owned()
            } else {
                report.computer.clone()
            };
            profile.name = computer;
            profile.created = clock::now_utc_iso();
            profile.modified = String::new();
            let mut doc = ProfileDoc::new(profile, None, catalog);
            doc.dirty = true;
            doc.issues = issues;
            self.open_tab(Doc::Profile(Box::new(doc)));
            let forms = if summary.forms.is_empty() {
                String::new()
            } else {
                format!("; forms: {}", summary.forms.join(", "))
            };
            self.status = format!("Read this PC: {}{forms}", summary.line());
        }
    }

    /// The result of a check of the rules of the tab named by `key`, listed in its messages.
    pub(crate) fn finish_check(
        &mut self,
        key: &str,
        plan: &thispc::Plan,
        report: &Report,
        readings: &[thispc::Reading],
    ) {
        let mut issues = Vec::new();
        if !report.admin && plan.admin_checks() > 0 {
            issues.push(admin_issue());
        }
        let Some(doc) = profile_doc(&mut self.docs, key) else {
            self.status = tr("Check on this PC: the tab of the profile was closed").to_owned();
            return;
        };
        let mut differ = 0;
        for r in readings {
            let on = doc.resolution.in_effect(&r.rule);
            let agrees = match r.verdict {
                Verdict::InEffect | Verdict::AsDefault => on || r.verdict == Verdict::AsDefault,
                Verdict::Partly | Verdict::NotInEffect => !on,
                Verdict::NotReadable => true,
            };
            if !agrees {
                differ += 1;
            }
            issues.push(Issue {
                severity: if agrees {
                    Severity::Info
                } else {
                    Severity::Warning
                },
                target: Target::Rule(r.rule.clone()),
                text: format!(
                    "This PC: {} ({} in the profile)",
                    r.verdict.title(),
                    if on { "on" } else { "off" }
                ),
            });
        }
        issues.sort_by_key(|i| i.severity);
        doc.issues = issues;
        self.status = trf(
            "Checked {n} rules on this PC: {differ} differ from the profile",
            &[("n", &readings.len()), ("differ", &differ)],
        );
    }

    /// Saves a stand-alone script for the selection into a folder the user picks; an apply or
    /// return script gets the undo script next to it.
    pub(crate) fn save_script(&mut self, kind: ScriptKind) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Save a script: the active tab is not a profile").to_owned();
            return;
        };
        let version = winweaver_core::BUILD;
        let stem = file_stem(&doc.profile.name);
        let made = match kind {
            ScriptKind::Apply => scripts::apply_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| (t, summary_of(&p), format!("Apply-{stem}.ps1"), true)),
            ScriptKind::Revert => {
                scripts::revert_script(&self.catalog, &doc.profile, &ids, version)
                    .map(|(t, p)| (t, summary_of(&p), format!("Revert-{stem}.ps1"), true))
            }
            ScriptKind::Audit => scripts::audit_script(&self.catalog, &doc.profile, &ids, version)
                .map(|(t, p)| {
                    (
                        t,
                        trf("{n} rules checked", &[("n", &p.rules.len())]),
                        format!("Audit-{stem}.ps1"),
                        false,
                    )
                }),
        };
        let (text, summary, name, with_undo) = match made {
            Ok(m) => m,
            Err(e) => {
                self.status = trf("Script not made: {e}", &[("e", &e)]);
                return;
            }
        };
        let undo = if with_undo {
            match scripts::undo_script(&doc.profile, version) {
                Ok(u) => Some(u),
                Err(e) => {
                    self.status = trf("Script not made: {e}", &[("e", &e)]);
                    return;
                }
            }
        } else {
            None
        };
        let start = output_dir();
        let dialog = rfd::FileDialog::new().set_title(tr("Choose the folder for the script"));
        let dialog = widgets::in_folder(dialog, &start);
        let Some(dir) = dialog.pick_folder() else {
            return;
        };
        let path = dir.join(&name);
        if let Err(e) = write_file(&path, &scripts::file_bytes(&text)) {
            self.status = trf("Script not saved: {e}", &[("e", &e)]);
            return;
        }
        if let Some(u) = undo
            && let Err(e) = write_file(&dir.join(scripts::UNDO_NAME), &scripts::file_bytes(&u))
        {
            self.status = trf("Undo script not saved: {e}", &[("e", &e)]);
            return;
        }
        self.status = trf(
            "Saved {path} ({summary}); run it as administrator, first on a test PC",
            &[("path", &path.display()), ("summary", &summary)],
        );
        self.shown_file = Some((self.status.clone(), path));
    }

    /// Saves the registry steps of the apply or the return of the selection as a registry file, as
    /// the scripts are saved; the steps a registry file cannot hold are comments in it.
    pub(crate) fn save_plan_reg(&mut self, revert: bool) {
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Save a registry file: the active tab is not a profile").to_owned();
            return;
        };
        let stem = file_stem(&doc.profile.name);
        let profile = &doc.profile.name;
        let (plan, title, name) = if revert {
            (
                winweaver_core::apply::plan_revert(&self.catalog, &ids, "reg"),
                format!("Return the rules of {profile} to the defaults of Windows"),
                format!("Revert-{stem}.{}", regfile::EXTENSION),
            )
        } else {
            (
                winweaver_core::apply::plan_apply(&self.catalog, &doc.profile, &ids, "reg"),
                format!("Apply the profile {profile}"),
                format!("Apply-{stem}.{}", regfile::EXTENSION),
            )
        };
        let file = RegFile::from_plan(&plan, &title);
        let values = file
            .lines
            .iter()
            .filter(|l| matches!(l, Line::Value { .. }))
            .count();
        if values == 0 {
            let text = tr("Nothing for a registry file: the selection writes no registry value");
            self.status = text.to_owned();
            return;
        }
        let start = output_dir();
        let dialog = rfd::FileDialog::new()
            .set_title(tr("Save the registry file"))
            .add_filter(tr("Registry files"), &[regfile::EXTENSION])
            .set_file_name(name);
        let dialog = widgets::in_folder(dialog, &start);
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = reg_extension(path);
        if let Err(e) = file.save(&path) {
            self.status = trf("Registry file not saved: {e}", &[("e", &e)]);
            return;
        }
        self.status = trf(
            "Saved {path} ({n} values); import it as administrator, first on a test PC",
            &[("path", &path.display()), ("n", &values)],
        );
        self.shown_file = Some((self.status.clone(), path));
    }

    /// Opens the report of an audit script made on another PC and reads it into a new profile.
    pub(crate) fn open_report(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Open an audit report"))
            .add_filter(tr("Audit report"), &["json"])
            .pick_file()
        else {
            return;
        };
        match Report::load(&path) {
            Ok(report) => self.open_report_from(report, &path),
            Err(e) => self.status = trf("Report not read: {e}", &[("e", &e)]),
        }
    }

    /// Reads an audit report into a new profile, on the base of the open profile.
    pub(crate) fn open_report_from(&mut self, report: Report, path: &Path) {
        let base = match self.docs.get(self.active) {
            Some(Doc::Profile(doc)) => doc.profile.clone(),
            _ => Profile::parse(PRESETS[0].1, PRESETS[0].0)
                .unwrap_or_else(|_| Profile::from_defaults(PRESETS[0].0, &self.catalog)),
        };
        let mut ids: Vec<String> = Vec::new();
        for r in &report.results {
            if !ids.contains(&r.rule) {
                ids.push(r.rule.clone());
            }
        }
        let plan = thispc::plan(&self.catalog, &base, &ids);
        let readings = thispc::judge(&self.catalog, &plan, &report.results);
        let (mut profile, summary) =
            thispc::capture(&self.catalog, &base, &readings, report.system.as_ref());
        profile.name = if report.computer.is_empty() {
            "Audit report".to_owned()
        } else {
            report.computer.clone()
        };
        let mut doc = ProfileDoc::new(profile, None, &self.catalog);
        doc.dirty = true;
        doc.issues = capture_issues(&summary);
        self.open_tab(Doc::Profile(Box::new(doc)));
        self.status = format!("Read the report {}: {}", path.display(), summary.line());
    }

    /// Prepares the native apply or return of the selection and asks for the confirmation.
    pub(crate) fn plan_native(&mut self, revert: bool) {
        if self.running.is_some() {
            self.status = tr("A change of this PC is running; wait for its report").to_owned();
            return;
        }
        let ids = self.selection_ids();
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Open a profile first").to_owned();
            return;
        };
        let nonce = match winweaver_mcp::new_token() {
            Ok(n) => n,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let plan = if revert {
            winweaver_core::apply::plan_revert(&self.catalog, &ids, &nonce)
        } else {
            winweaver_core::apply::plan_apply(&self.catalog, &doc.profile, &ids, &nonce)
        };
        if plan.steps.is_empty() {
            self.status = trf(
                "Nothing to change natively; {n} steps are left to the stand-alone script",
                &[("n", &plan.skipped.len())],
            );
            return;
        }
        self.confirm = Some(plan);
    }

    /// Prepares the undo of a backup file the user picks.
    pub(crate) fn plan_undo(&mut self) {
        let start = output_dir();
        let dialog = rfd::FileDialog::new()
            .set_title(tr("Choose the backup to restore"))
            .add_filter(tr("Backups"), &["json"]);
        let dialog = widgets::in_folder(dialog, &start);
        let Some(path) = dialog.pick_file() else {
            return;
        };
        let entries = match winweaver_core::apply::read_backup(&path) {
            Ok(e) => e,
            Err(e) => {
                self.status = trf("Backup not read: {e}", &[("e", &e)]);
                return;
            }
        };
        let nonce = match winweaver_mcp::new_token() {
            Ok(n) => n,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let plan = winweaver_core::apply::plan_undo(&entries, &nonce);
        if plan.steps.is_empty() {
            self.status = tr("The backup holds nothing to restore").to_owned();
            return;
        }
        self.confirm = Some(plan);
    }

    /// Writes the confirmed plan into the output folder and runs the elevated helper on it in a
    /// thread; the report comes back through a channel.
    pub(crate) fn run_plan(&mut self) {
        let Some(plan) = self.confirm.take() else {
            return;
        };
        let dir = output_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.status = trf("The plan was not written: {e}", &[("e", &e)]);
            return;
        }
        let stamp = clock::now_utc_iso()
            .replace([':', '-'], "")
            .replace('T', "-")
            .trim_end_matches('Z')
            .to_owned();
        let path = dir.join(format!("plan-{stamp}.json"));
        let text = plan.to_json();
        let digest = winweaver_core::apply::digest(text.as_bytes());
        if let Err(e) = write_file(&path, text.as_bytes()) {
            self.status = trf("The plan was not written: {e}", &[("e", &e)]);
            return;
        }
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        self.running_skipped = plan.skipped.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let args = vec![
                "--elevated-apply".to_owned(),
                path.display().to_string(),
                digest,
            ];
            let result = winweaver_win::run_elevated(&exe, &args).and_then(|code| {
                let report_path = path.with_file_name(format!("plan-{stamp}-report.json"));
                let read = std::fs::read_to_string(&report_path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| {
                        serde_json::from_str::<winweaver_core::apply::Report>(&t)
                            .map_err(|e| e.to_string())
                    });
                let _ = std::fs::remove_file(&report_path);
                match (code, read) {
                    (0, Ok(report)) => Ok(report),
                    (0, Err(e)) => Err(e),
                    (code, _) => Err(format!("the helper ended with the code {code}")),
                }
            });
            let _ = std::fs::remove_file(&path);
            let _ = tx.send(result);
        });
        self.running = Some(rx);
        self.status = tr("Windows asks for the permission; the change of this PC runs").to_owned();
    }

    /// Takes the report of a finished helper.
    pub(crate) fn poll_running(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.running else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.running = None;
                let skipped = std::mem::take(&mut self.running_skipped);
                match result {
                    Ok(report) => {
                        let mut issues: Vec<Issue> = report
                            .results
                            .iter()
                            .filter(|r| r.status != "ok")
                            .map(|r| Issue {
                                severity: if r.status == "failed" {
                                    Severity::Warning
                                } else {
                                    Severity::Info
                                },
                                target: Target::Rule(r.rule.clone()),
                                text: format!("{}: {} {}", r.status, r.what, r.note),
                            })
                            .collect();
                        issues.extend(skipped.into_iter().map(|(rule, why)| Issue {
                            severity: Severity::Info,
                            target: Target::Rule(rule),
                            text: trf("Left to the stand-alone script: {why}", &[("why", &why)]),
                        }));
                        if let Some(Doc::Profile(doc)) = self.docs.get_mut(self.active) {
                            doc.issues = issues;
                        }
                        self.status = trf(
                            "This PC changed: {ok} done, {failed} failed, {skipped} skipped; backup {backup}",
                            &[
                                ("ok", &report.count("ok")),
                                ("failed", &report.count("failed")),
                                ("skipped", &report.count("skipped")),
                                (
                                    "backup",
                                    &if report.backup.is_empty() {
                                        "-".to_owned()
                                    } else {
                                        report.backup.clone()
                                    },
                                ),
                            ],
                        );
                    }
                    Err(e) => self.status = trf("This PC was not changed: {e}", &[("e", &e)]),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.running = None,
        }
    }

    /// The confirmation of a plan, over the window.
    pub(crate) fn confirm_view(&self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(plan) = &self.confirm else {
            return;
        };
        let title = match plan.purpose.as_str() {
            "revert" => tr("Return to Windows defaults"),
            "undo" => tr("Undo a change"),
            _ => tr("Apply to this PC"),
        };
        widgets::Modal::new(title).show(ctx, |ui| {
                ui.label(trf(
                    "{n} changes of this PC run as administrator; Windows asks for the permission first.",
                    &[("n", &plan.changes())],
                ));
                if !plan.skipped.is_empty() {
                    ui.label(trf(
                        "{n} steps are left to the stand-alone script (scripts, the profiles of other users, values without a known Windows default).",
                        &[("n", &plan.skipped.len())],
                    ));
                }
                if plan.uses_default_user() {
                    ui.label(tr("The default user profile changes too: accounts created from now on get its settings."));
                }
                if plan.removals() > 0 {
                    ui.label(trf(
                        "{n} steps remove apps or capabilities of Windows; Undo does not bring them back.",
                        &[("n", &plan.removals())],
                    ));
                }
                if plan.irreversible() > 0 {
                    ui.label(trf(
                        "{n} steps run programs, change accounts or delete copies of the answer file; Undo does not take them back.",
                        &[("n", &plan.irreversible())],
                    ));
                }
                ui.label(tr("Every registry value is saved to a backup in the output folder before its change; Undo restores it."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("Change this PC")).clicked() {
                        acts.push(Act::Pc(PcAct::Confirm));
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        acts.push(Act::Pc(PcAct::Cancel));
                    }
                });
            });
    }

    /// Runs an act of the acts of This PC: the read, the check, the scripts, the native apply.
    pub(crate) fn apply_pc(&mut self, act: PcAct) {
        match act {
            PcAct::Read => self.read_pc(false),
            PcAct::ReadSelection => self.read_pc(true),
            PcAct::Check => self.check_on_pc(),
            PcAct::SaveScript(kind) => self.save_script(kind),
            PcAct::SaveRegFile(revert) => self.save_plan_reg(revert),
            PcAct::PlanNative(revert) => self.plan_native(revert),
            PcAct::PlanUndo => self.plan_undo(),
            PcAct::Confirm => self.run_plan(),
            PcAct::Cancel => self.confirm = None,
        }
    }
}
