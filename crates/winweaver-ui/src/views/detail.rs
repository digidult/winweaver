//! The detail panel: the profile, a group, the unknown options, and the panel that picks by the selection.

use super::*;

pub(crate) fn profile_detail(ui: &mut egui::Ui, doc: &ProfileDoc) {
    let p = &doc.profile;
    header(ui, |ui| {
        ui.label(style::strong(ui, p.name.as_str(), 18.0));
        ui.horizontal_wrapped(|ui| {
            style::chip(
                ui,
                &trf("Catalog {version}", &[("version", &p.catalog)]),
                true,
            );
        });
        if !p.comment.is_empty() {
            ui.add_space(4.0);
            style::selectable_text(ui, "profile-comment", &p.comment);
        }
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Installation"));
        ui.label(trf(
            "Edition {edition}, product key {key}, time zone {zone}, computer name {name}, accounts {accounts}, disk {disk}",
            &[
                ("edition", &p.install.edition),
                ("key", &p.install.product_key_mode),
                ("zone", &p.install.time_zone),
                ("name", &p.install.computer_name_mode),
                ("accounts", &p.install.account_mode),
                ("disk", &p.install.disk_mode),
            ],
        ));
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Languages"));
        ui.label(trf(
            "Display {ui}, system {system}, user {user}, input {input}",
            &[
                ("ui", &p.languages.ui_language),
                ("system", &p.languages.system_locale),
                ("user", &p.languages.user_locale),
                ("input", &p.languages.input.join(", ")),
            ],
        ));
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Accounts"));
        for a in &p.accounts {
            ui.label(format!("{} ({})", a.name, a.group));
        }
    });
    ui.add_space(10.0);
    egui::Frame::new()
        .inner_margin(Margin::symmetric(18, 0))
        .show(ui, |ui| {
            ui.weak(tr("Select a form, a group or a rule in the tree."));
        });
}

/// The button that looks for the catalogs of the unknown options among the imports.
pub(crate) fn load_catalogs_button(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    if ui
        .button(tr("Load catalogs"))
        .on_hover_text(tr(
            "Looks for these options in the imported catalogs that are not loaded and loads only the ones that have them",
        ))
        .clicked()
    {
        acts.push(Act::Import(ImportAct::LoadCatalogs));
    }
}

/// The group of the options of catalogs that are not loaded.
pub(crate) fn unknown_group_detail(ui: &mut egui::Ui, doc: &ProfileDoc, acts: &mut Vec<Act>) {
    let unknown = &doc.profile.unknown;
    header(ui, |ui| {
        ui.label(style::strong(ui, tr("Unknown options"), 18.0));
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, &trf("{n} options", &[("n", &unknown.len())]), true);
        });
        ui.add_space(8.0);
        load_catalogs_button(ui, acts);
    });
    block(ui, |ui| {
        style::selectable_text(
            ui,
            "unknown-summary",
            tr(
                "These options of the profile belong to catalogs of policy templates that are not loaded. They stay in the profile unchanged and are left out of the build. Load catalogs looks for them in the imported catalogs that are not loaded and loads only the ones that have them.",
            ),
        )
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Options"));
        for (id, choice) in unknown {
            ui.horizontal(|ui| {
                ui.label(RichText::new(id).monospace());
                ui.weak(if unknown_on(choice) {
                    tr("on")
                } else {
                    tr("off")
                });
                if ui.small_button(tr("open")).clicked() {
                    acts.push(Act::Profile(ProfileAct::Select(Sel::Unknown(id.clone()))));
                }
            });
        }
    });
}

/// An option of the profile whose catalog is not loaded: its id and its saved choice.
pub(crate) fn unknown_detail(ui: &mut egui::Ui, doc: &ProfileDoc, id: &str, acts: &mut Vec<Act>) {
    let Some(choice) = doc.profile.unknown.get(id) else {
        return;
    };
    header(ui, |ui| {
        ui.label(style::strong(ui, tr("Unknown option"), 18.0));
        id_line(ui, id);
        ui.horizontal_wrapped(|ui| {
            let on = unknown_on(choice);
            style::chip(ui, if on { tr("On") } else { tr("Off") }, on);
        });
        ui.add_space(8.0);
        load_catalogs_button(ui, acts);
    });
    block(ui, |ui| {
        style::selectable_text(
            ui,
            ("unknown-option", id),
            tr(
                "This option belongs to a catalog of policy templates that is not loaded. It stays in the profile unchanged and is left out of the build.",
            ),
        )
    });
    if let Some(params) = choice.get("params").and_then(Value::as_object)
        && !params.is_empty()
    {
        block(ui, |ui| {
            style::section_title(ui, tr("Parameters"));
            for (name, value) in params {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(name).monospace());
                    ui.weak(value_text(value));
                });
            }
        });
    }
}

pub(crate) fn group_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(group) = catalog.group(id) else {
        return;
    };
    let rules = catalog.rules_under(id);
    let on = rules
        .iter()
        .filter(|r| doc.resolution.shown_on(&r.id))
        .count();
    header(ui, |ui| {
        ui.label(style::strong(ui, gtitle(texts, group), 18.0));
        id_line(ui, &group.id);
        ui.horizontal_wrapped(|ui| {
            style::chip(
                ui,
                &trf(
                    "{n} rules, {on} in effect",
                    &[("n", &rules.len()), ("on", &on)],
                ),
                true,
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(tr("Turn all on")).clicked() {
                acts.push(Act::Profile(ProfileAct::SetGroup(id.to_owned(), true)));
            }
            if ui.button(tr("Turn all off")).clicked() {
                acts.push(Act::Profile(ProfileAct::SetGroup(id.to_owned(), false)));
            }
        });
    });
    let summary = gsummary(texts, group);
    if !summary.is_empty() {
        block(ui, |ui| {
            style::selectable_text(ui, ("group-summary", id), summary)
        });
    }
    // The group above and the groups in it, as links.
    let parent = group.parent.as_deref().and_then(|p| catalog.group(p));
    let children = catalog.child_groups(Some(id));
    if parent.is_some() || !children.is_empty() {
        block(ui, |ui| {
            if let Some(p) = parent {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(tr("In the group"));
                    if ui.link(gtitle(texts, p)).clicked() {
                        acts.push(Act::Profile(ProfileAct::Reveal(Sel::Group(p.id.clone()))));
                    }
                });
            }
            if !children.is_empty() {
                style::section_title(ui, tr("Groups in it"));
                ui.add_space(4.0);
            }
            for g in children {
                ui.horizontal_wrapped(|ui| {
                    if ui.link(gtitle(texts, g)).clicked() {
                        acts.push(Act::Profile(ProfileAct::Reveal(Sel::Group(g.id.clone()))));
                    }
                    let n = catalog.rules_under(&g.id).len();
                    ui.weak(trf("{n} rules", &[("n", &n)]));
                });
            }
        });
    }
    // The passes of the installation and the first sign-in have nothing for a running PC.
    let ids: Vec<String> = catalog
        .rules_under(id)
        .iter()
        .map(|r| r.id.clone())
        .collect();
    if winweaver_core::export::applicable(catalog, &ids) {
        block(ui, |ui| {
            style::section_title(ui, tr("This PC, this selection"));
            ui.add_space(4.0);
            this_pc_buttons(ui, acts);
        });
    }
    block(ui, |ui| {
        style::section_title(ui, tr("Rules of this group"));
        for rule in catalog.rules_in(id) {
            ui.horizontal(|ui| {
                rule_checkbox(ui, catalog, texts, doc, rule, rtitle(texts, rule), acts);
                if ui.small_button(tr("open")).clicked() {
                    // In this group, also for a policy another import holds first.
                    let sel = Sel::Rule(rule.id.clone());
                    acts.push(Act::Profile(ProfileAct::SelectIn(sel, Some(id.to_owned()))));
                }
            });
        }
    });
}

/// The commands of This PC for the selected rule or group.
pub(crate) fn this_pc_buttons(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    ui.horizontal_wrapped(|ui| {
        for (text, act) in [
            (tr("Check on this PC"), Act::Pc(PcAct::Check)),
            (tr("Read into the profile"), Act::Pc(PcAct::ReadSelection)),
            (tr("Apply to this PC..."), Act::Pc(PcAct::PlanNative(false))),
            (
                tr("Return to Windows defaults..."),
                Act::Pc(PcAct::PlanNative(true)),
            ),
            (
                tr("Save an apply script..."),
                Act::Pc(PcAct::SaveScript(ScriptKind::Apply)),
            ),
            (
                tr("Save a return-to-defaults script..."),
                Act::Pc(PcAct::SaveScript(ScriptKind::Revert)),
            ),
            (
                tr("Save a registry file..."),
                Act::Pc(PcAct::SaveRegFile(false)),
            ),
            (
                tr("Save a return-to-defaults registry file..."),
                Act::Pc(PcAct::SaveRegFile(true)),
            ),
        ] {
            if ui.button(text).clicked() {
                acts.push(act);
            }
        }
    });
}

pub fn detail_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    doc: &ProfileDoc,
    settings: &Settings,
    acts: &mut Vec<Act>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| match &doc.sel {
            Sel::None => profile_detail(ui, doc),
            Sel::Group(id) if id == UNKNOWN_GROUP => unknown_group_detail(ui, doc, acts),
            Sel::Group(id) => group_detail(ui, catalog, texts, doc, id, acts),
            Sel::Unknown(id) => unknown_detail(ui, doc, id, acts),
            Sel::Rule(id) => rule_detail(ui, catalog, texts, imports, doc, settings, id, acts),
            Sel::Form(form) => {
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        crate::forms::form_view(ui, doc, *form, &crate::i18n::current(), acts);
                    });
            }
        });
}
