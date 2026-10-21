//! The diffs: comparing profiles, the diff tab, its items applied.

use crate::*;

impl App {
    /// Saves the active diff to its file, or asks for one.
    pub(crate) fn save_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        let Some(path) = d.path.clone() else {
            self.save_active_diff();
            return;
        };
        match d.diff.save(&path) {
            Ok(()) => {
                d.dirty = false;
                self.saved(&path);
            }
            Err(e) => self.save_failed(e),
        }
    }

    /// The differences of the profile in front from its file, or from the preset it came from
    /// when it has no file yet.
    pub(crate) fn diff_unsaved(&mut self) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        let (saved, path) = match &doc.path {
            Some(path) => match Profile::load(path) {
                Ok(saved) => (saved, Some(path.clone())),
                Err(e) => {
                    self.status = trf("Compare: {e}", &[("e", &e)]);
                    return;
                }
            },
            None => {
                let preset = PRESETS.iter().find(|p| p.0 == doc.profile.name);
                match preset.map(|(name, text)| Profile::parse(text, name)) {
                    Some(Ok(saved)) => (saved, None),
                    Some(Err(e)) => {
                        self.status = trf("Compare: {e}", &[("e", &e)]);
                        return;
                    }
                    None => {
                        self.status =
                            tr("The profile has no file and no preset to compare with").to_owned();
                        return;
                    }
                }
            }
        };
        self.compare_active(saved, path.as_deref());
    }

    pub(crate) fn open_diff_path(&mut self, path: PathBuf) {
        match Diff::load(&path) {
            Ok(diff) => {
                self.docs
                    .push(Doc::Diff(Box::new(DiffDoc::new(diff, Some(path.clone())))));
                self.active = self.docs.len() - 1;
                self.opened(&path);
            }
            Err(e) => self.open_failed(e),
        }
    }

    pub(crate) fn compare_active(&mut self, other: Profile, other_path: Option<&Path>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            self.status = tr("Compare: the active tab is not a profile").to_owned();
            return;
        };
        let base_side = diff::Side::of(&doc.profile, doc.path.as_deref());
        let other_side = diff::Side::of(&other, other_path);
        let diff = Diff::between(&doc.profile, &other, base_side, other_side);
        self.status = trf("{n} differences", &[("n", &diff.count())]);
        self.docs
            .push(Doc::Diff(Box::new(DiffDoc::new(diff, None))));
        self.active = self.docs.len() - 1;
    }

    /// The profile tab a diff belongs to: the one with the path of its base, else the one with
    /// its name, else the first profile tab.
    pub(crate) fn diff_target(&self, diff: &Diff) -> Option<usize> {
        let by_path = |d: &Doc| {
            matches!(d, Doc::Profile(p) if !diff.base.path.is_empty()
                && p.path.as_ref().is_some_and(|x| x.display().to_string() == diff.base.path))
        };
        let by_name = |d: &Doc| matches!(d, Doc::Profile(p) if p.profile.name == diff.base.name);
        self.docs
            .iter()
            .position(by_path)
            .or_else(|| self.docs.iter().position(by_name))
            .or_else(|| self.docs.iter().position(|d| matches!(d, Doc::Profile(_))))
    }

    pub(crate) fn apply_active_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let Some(i) = self.diff_target(&diff) else {
            self.status = tr("Apply: open the profile to change first").to_owned();
            return;
        };
        let catalog = &self.catalog;
        if let Some(Doc::Profile(p)) = self.docs.get_mut(i) {
            diff.apply_to(&mut p.profile);
            p.refresh(catalog);
            self.status = trf(
                "Applied {n} differences to {profile}",
                &[("n", &diff.count()), ("profile", &p.title())],
            );
        }
        self.active = i;
    }

    /// Shows an item of the active diff in the profile tab of a side; the profile is opened from
    /// its file, or from the preset of its name, when no tab has it.
    pub(crate) fn go_to_diff_item(&mut self, key: &str, other: bool) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let base_tab = self.diff_target(&diff);
        let tab = if other {
            let side = &diff.other;
            let by_path = |(i, d): (usize, &Doc)| {
                Some(i) != base_tab
                    && matches!(d, Doc::Profile(p) if !side.path.is_empty()
                        && p.path.as_ref().is_some_and(|x| x.display().to_string() == side.path))
            };
            let by_name = |(i, d): (usize, &Doc)| {
                Some(i) != base_tab && matches!(d, Doc::Profile(p) if p.profile.name == side.name)
            };
            self.docs
                .iter()
                .enumerate()
                .position(by_path)
                .or_else(|| self.docs.iter().enumerate().position(by_name))
        } else {
            base_tab
        };
        let side = if other { &diff.other } else { &diff.base };
        match tab {
            Some(i) => self.active = i,
            None if !side.path.is_empty() && Path::new(&side.path).is_file() => {
                self.open_path(PathBuf::from(&side.path));
            }
            None => match PRESETS.iter().find(|p| p.0 == side.name) {
                Some(&(name, text)) => {
                    let profile = Profile::parse(text, name)
                        .unwrap_or_else(|_| Profile::from_defaults(name, &self.catalog));
                    let doc = ProfileDoc::new(profile, None, &self.catalog);
                    self.open_tab(Doc::Profile(Box::new(doc)));
                }
                None => {
                    self.status = trf(
                        "The profile {name} is not open and has no file to open",
                        &[("name", &side.name)],
                    );
                    return;
                }
            },
        }
        let sel = sel_from_diff_key(&self.catalog, key);
        if let Some(doc) = self.active_profile() {
            doc.select(sel);
        }
        self.reveal_selection();
    }

    /// Sets one item of the active diff in the profile of its base and stays on the diff.
    pub(crate) fn apply_diff_item(&mut self, key: &str) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            return;
        };
        let diff = d.diff.clone();
        let Some(i) = self.diff_target(&diff) else {
            self.status = tr("Apply: open the profile to change first").to_owned();
            return;
        };
        let catalog = &self.catalog;
        if let Some(Doc::Profile(p)) = self.docs.get_mut(i) {
            diff.apply_item(key, &mut p.profile);
            p.refresh(catalog);
            self.status = trf("Applied one difference to {name}", &[("name", &p.title())]);
        }
    }

    pub(crate) fn save_active_diff(&mut self) {
        let Some(Doc::Diff(d)) = self.docs.get(self.active) else {
            self.status = tr("Save as a diff file: the active tab is not a diff").to_owned();
            return;
        };
        let proposed = format!(
            "{}-vs-{}.{}",
            file_stem(&d.diff.base.name),
            file_stem(&d.diff.other.name),
            diff::EXTENSION
        );
        let Some(path) = file_dialog("WinWeaver diff", diff::EXTENSION)
            .set_title(tr("Save the diff as"))
            .set_file_name(proposed)
            .save_file()
        else {
            return;
        };
        let path = if path.extension().is_some() {
            path
        } else {
            path.with_extension(diff::EXTENSION)
        };
        let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) else {
            return;
        };
        match d.diff.save(&path) {
            Ok(()) => {
                d.path = Some(path.clone());
                d.dirty = false;
                self.saved(&path);
                self.remember_recent(&path);
            }
            Err(e) => self.save_failed(e),
        }
    }

    /// Runs an act of the acts of the comparison of profiles and of the diff tab.
    pub(crate) fn apply_diff(&mut self, act: DiffAct) {
        match act {
            DiffAct::Unsaved => self.diff_unsaved(),
            DiffAct::Compare => {
                if let Some(path) = pick_profile() {
                    match Profile::load(&path) {
                        Ok(other) => self.compare_active(other, Some(&path)),
                        Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                    }
                }
            }
            DiffAct::CompareWith(name) => {
                if let Some((name, text)) = PRESETS.iter().find(|p| p.0 == name) {
                    match Profile::parse(text, name) {
                        Ok(other) => self.compare_active(other, None),
                        Err(e) => self.status = trf("Compare: {e}", &[("e", &e)]),
                    }
                }
            }
            DiffAct::Apply => self.apply_active_diff(),
            DiffAct::SaveAs => self.save_active_diff(),
            DiffAct::Select(key) => {
                if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                    d.sel = Some(key);
                }
            }
            DiffAct::Comment(key, text) => {
                if let Some(Doc::Diff(d)) = self.docs.get_mut(self.active) {
                    if text.trim().is_empty() {
                        d.diff.comments.shift_remove(&key);
                    } else {
                        d.diff.comments.insert(key, text);
                    }
                    d.dirty = true;
                }
            }
            DiffAct::GoTo(key, other) => self.go_to_diff_item(&key, other),
            DiffAct::ApplyItem(key) => self.apply_diff_item(&key),
        }
    }
}
