//! The detail of a rule: its cards, its parameters, its description with the links to what it writes and to other catalogs.

use super::*;

/// What a rule writes: the registry values as a table of key, name, value and the default of
/// Windows; the other actions as one line each under it.
pub(crate) fn writes(ui: &mut egui::Ui, rule: &Rule, acts: &mut Vec<Act>) {
    let p = style::palette(ui);
    let mut rows = Vec::new();
    let mut others = Vec::new();
    for action in &rule.actions {
        match action {
            Action::Reg {
                path,
                name,
                value,
                default,
                ..
            } => rows.push((
                path.as_str(),
                name.clone(),
                value_text(value),
                default_text(default),
            )),
            Action::RegRemove {
                path,
                name,
                default,
            } => rows.push((
                path.as_str(),
                name.clone(),
                tr("removed").to_owned(),
                default_text(default),
            )),
            Action::RegList { path, param, .. } => rows.push((
                path.as_str(),
                String::new(),
                trf("the items of the parameter {param}", &[("param", param)]),
                String::new(),
            )),
            other => others.push(describe(other)),
        }
    }
    if !rows.is_empty() {
        let room = (ui.available_width() - 3.0 * 16.0).max(240.0);
        let widths = [room * 0.4, room * 0.22, room * 0.22, room * 0.16];
        let cell = |ui: &mut egui::Ui, text: &str, mono: bool, muted: bool, width: f32| {
            ui.scope(|ui| {
                ui.set_max_width(width);
                let mut rich = RichText::new(text);
                rich = if mono {
                    rich.font(style::mono(ui, 12.0))
                } else {
                    rich.font(style::font(ui, 12.5))
                };
                if muted {
                    rich = rich.color(p.muted);
                }
                style::copy_label(ui, rich, true);
            });
        };
        egui::Grid::new(("writes", rule.id.as_str()))
            .num_columns(4)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (title, width) in [tr("Key"), tr("Name"), tr("Value"), tr("Windows default")]
                    .into_iter()
                    .zip(widths)
                {
                    ui.scope(|ui| {
                        ui.set_max_width(width);
                        ui.add(
                            egui::Label::new(style::strong(ui, title, 12.0).color(p.muted)).wrap(),
                        );
                    });
                }
                ui.end_row();
                for (path, name, value, default) in &rows {
                    ui.horizontal(|ui| {
                        cell(ui, path, true, false, widths[0] - 24.0);
                        let open = ui
                            .small_button(icons::GO_TO)
                            .on_hover_text(tr("Open in the registry"));
                        if open.clicked() {
                            let value = (!name.is_empty()).then(|| name.clone());
                            acts.push(Act::Registry(RegistryAct::Open((*path).to_owned(), value)));
                        }
                    });
                    cell(ui, name, false, false, widths[1]);
                    cell(ui, value, false, false, widths[2]);
                    cell(ui, default, false, true, widths[3]);
                    ui.end_row();
                }
            });
    }
    for line in others {
        ui.add_space(4.0);
        style::copy_label(ui, RichText::new(line).font(style::mono(ui, 12.0)), true);
    }
}

/// The same policy in the trees of other imports and the policies of the same name in other
/// catalogs, as links that select them.
pub(crate) fn other_catalogs(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    rule: &Rule,
    acts: &mut Vec<Act>,
) {
    if !rule.id.starts_with("admx.") {
        return;
    }
    let aliases: Vec<&str> = catalog.alias_groups(&rule.id).collect();
    let name = policy_name(&rule.id);
    let off = rule.id.ends_with(".off");
    let similar: Vec<&Rule> = catalog
        .rules()
        .iter()
        .filter(|r| {
            r.id.starts_with("admx.")
                && r.id != rule.id
                && r.id.ends_with(".off") == off
                && policy_name(&r.id) == name
        })
        .collect();
    if aliases.is_empty() && similar.is_empty() {
        return;
    }
    block(ui, |ui| {
        style::section_title(ui, tr("In other catalogs"));
        ui.add_space(4.0);
        for group in aliases {
            ui.horizontal_wrapped(|ui| {
                ui.weak(tr("The same policy in"));
                if ui.link(import_of(catalog, texts, group)).clicked() {
                    acts.push(Act::Profile(ProfileAct::ShowIn(
                        rule.id.clone(),
                        group.to_owned(),
                    )));
                }
            });
        }
        for other in similar {
            ui.horizontal_wrapped(|ui| {
                if ui.link(rtitle(texts, other)).clicked() {
                    acts.push(Act::Profile(ProfileAct::Reveal(Sel::Rule(
                        other.id.clone(),
                    ))));
                }
                // The import, a link to its catalog tab at the policy.
                let holds = |f: &&CatalogFile| f.rules.iter().any(|r| r.id == other.id);
                match imports.iter().find(holds) {
                    Some(f) => {
                        if ui.link(import_of(catalog, texts, &other.group)).clicked() {
                            acts.push(Act::Catalog(CatalogAct::OpenAt(
                                f.id.clone(),
                                other.id.clone(),
                            )));
                        }
                    }
                    None => {
                        ui.weak(import_of(catalog, texts, &other.group));
                    }
                }
            });
        }
    });
}

/// Links that open the loaded imports holding a rule in their catalog tabs at it.
pub(crate) fn catalog_links(
    ui: &mut egui::Ui,
    imports: &[CatalogFile],
    rule: &str,
    acts: &mut Vec<Act>,
) {
    for f in imports
        .iter()
        .filter(|f| f.rules.iter().any(|r| r.id == rule))
    {
        if ui
            .link(trf("Show in the catalog {name}", &[("name", &f.name)]))
            .clicked()
        {
            acts.push(Act::Catalog(CatalogAct::OpenAt(
                f.id.clone(),
                rule.to_owned(),
            )));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn rule_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    doc: &ProfileDoc,
    settings: &Settings,
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = catalog.rule(id) else {
        return;
    };
    header(ui, |ui| {
        ui.label(style::strong(ui, rtitle(texts, rule), 18.0));
        id_line(ui, &rule.id);
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, tr(mechanism_title(Catalog::mechanism(rule))), true);
            style::chip(ui, phase_title(rule.phase), false);
            style::chip(ui, tr(level_title(rule.level)), false);
            if rule.default {
                style::chip(ui, tr("on by default"), false);
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let mut on = doc.profile.is_on(&rule.id);
            if switch(ui, &mut on).changed() {
                acts.push(Act::Profile(ProfileAct::Toggle(rule.id.clone(), on)));
            }
            ui.label(if on { tr("On") } else { tr("Off") });
            ui.add_space(8.0);
            match doc.resolution.state(&rule.id) {
                State::On => {
                    ui.label(style::muted(ui, tr("in effect"), 12.0));
                }
                State::Off => match doc.resolution.linked_by(&rule.id) {
                    Some(by) => {
                        ui.label(style::muted(ui, tr("set by the built-in rule"), 12.0));
                        let title = catalog.rule(by).map_or(by, |r| rtitle(texts, r));
                        if ui.link(title).clicked() {
                            acts.push(Act::Profile(ProfileAct::Reveal(Sel::Rule(by.to_owned()))));
                        }
                    }
                    None => {
                        ui.label(style::muted(ui, tr("off"), 12.0));
                    }
                },
                State::Blocked { by } => {
                    ui.label(style::muted(ui, tr("blocked: needs"), 12.0));
                    let title = catalog.rule(by).map_or(by.as_str(), |r| rtitle(texts, r));
                    if ui.link(title).clicked() {
                        acts.push(Act::Profile(ProfileAct::Reveal(Sel::Rule(by.clone()))));
                    }
                }
            }
        });
    });
    if !rule.params.is_empty() {
        block(ui, |ui| {
            style::section_title(ui, tr("Parameters"));
            ui.add_space(4.0);
            egui::Grid::new(("params", rule.id.as_str()))
                .num_columns(3)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    for (name, param) in &rule.params {
                        param_editor(ui, catalog, texts, doc, rule, name, param, acts);
                        ui.end_row();
                    }
                });
        });
    }
    block(ui, |ui| {
        style::section_title(ui, tr("This PC, this rule"));
        ui.add_space(4.0);
        this_pc_buttons(ui, acts);
        catalog_links(ui, imports, &rule.id, acts);
    });
    other_catalogs(ui, catalog, texts, imports, rule, acts);
    rule_description(ui, catalog, texts, rule, settings, RuleLink::Profile, acts);
}

/// The description of a rule under its title, folded with Ctrl+D: what it does, the risk, what it
/// writes, the rules it needs or conflicts with and the ones that need it (links that lead where
/// `link` says), versions and the reference.
pub(crate) fn rule_description(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    rule: &Rule,
    settings: &Settings,
    link: RuleLink,
    acts: &mut Vec<Act>,
) {
    let open = settings.desc_open;
    let parts = &settings.desc_sections;
    ui.add_space(6.0);
    egui::Frame::new()
        .inner_margin(Margin {
            left: 20,
            right: 18,
            top: 8,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                style::section_title(ui, tr("Description"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (icon, hint) = if open {
                        (icons::UNFOLDED, tr("Fold the description (Ctrl+D)"))
                    } else {
                        (icons::FOLDED, tr("Show the description (Ctrl+D)"))
                    };
                    if style::icon_button(ui, icon, hint, true, None).clicked() {
                        acts.push(Act::Nav(NavAct::Fold));
                    }
                });
            });
        });
    if !open {
        return;
    }
    section(ui, "desc-what", tr("What it does"), parts.what, |ui| {
        style::selectable_text(ui, ("what", rule.id.as_str()), rsummary(texts, rule));
        if !rule.effect.is_empty() {
            style::selectable_text(ui, ("effect", rule.id.as_str()), &rule.effect);
        }
    });
    if !rule.risk.is_empty() {
        section(ui, "desc-risk", tr("Risk"), parts.risk, |ui| {
            style::selectable_text(ui, ("risk", rule.id.as_str()), &rule.risk);
        });
    }
    section(
        ui,
        "desc-writes",
        tr("What it writes"),
        parts.writes,
        |ui| writes(ui, rule, acts),
    );
    // The rules it names and the rules that name it, both ways.
    let requires: Vec<&str> = rule.requires.iter().map(String::as_str).collect();
    let mut needed_by = Vec::new();
    let mut conflicts: Vec<&str> = rule.conflicts.iter().map(String::as_str).collect();
    for other in catalog.rules() {
        if other.requires.contains(&rule.id) {
            needed_by.push(other.id.as_str());
        }
        if other.conflicts.contains(&rule.id) && !conflicts.contains(&other.id.as_str()) {
            conflicts.push(other.id.as_str());
        }
    }
    if !requires.is_empty() || !needed_by.is_empty() || !conflicts.is_empty() {
        section(
            ui,
            "desc-deps",
            tr("Dependencies and conflicts"),
            parts.deps,
            |ui| {
                for (label, list) in [
                    (tr("Needs"), &requires),
                    (tr("Needed by"), &needed_by),
                    (tr("Conflicts with"), &conflicts),
                ] {
                    for other in list {
                        ui.horizontal(|ui| {
                            ui.weak(label);
                            let title = catalog.rule(other).map_or(*other, |r| rtitle(texts, r));
                            if ui.link(title).on_hover_text(*other).clicked() {
                                acts.push(link.act(other));
                            }
                        });
                    }
                }
            },
        );
    }
    if !rule.versions.is_empty() || !rule.rollback.is_empty() || !rule.verify.is_empty() {
        section(
            ui,
            "desc-more",
            tr("Versions, check and rollback"),
            parts.more,
            |ui| {
                for (i, text) in [&rule.versions, &rule.verify, &rule.rollback]
                    .into_iter()
                    .enumerate()
                {
                    if !text.is_empty() {
                        style::selectable_text(ui, ("more", rule.id.as_str(), i), text);
                    }
                }
            },
        );
    }
    if !rule.doc.is_empty() {
        section(ui, "desc-ref", tr("Reference"), parts.reference, |ui| {
            if ui.link(rule.doc.as_str()).clicked() {
                acts.push(Act::Nav(NavAct::OpenDoc(rule.doc.clone())));
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn param_editor(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    rule: &Rule,
    name: &str,
    param: &Param,
    acts: &mut Vec<Act>,
) {
    let current = doc
        .profile
        .param(catalog, &rule.id, name)
        .unwrap_or(Value::Null);
    ui.label(ptitle(texts, rule, name));
    match param {
        Param::Enum { values, .. } => {
            let shown = values
                .iter()
                .find(|v| check::same_value(&v.value, &current))
                .map_or_else(|| current.to_string(), |v| v.title.clone());
            egui::ComboBox::from_id_salt(format!("{}.{name}", rule.id))
                .selected_text(shown)
                .width((ui.available_width() - 120.0).max(160.0))
                .show_ui(ui, |ui| {
                    for v in values {
                        let chosen = check::same_value(&v.value, &current);
                        if ui.selectable_label(chosen, &v.title).clicked() && !chosen {
                            acts.push(Act::Profile(ProfileAct::SetParam(
                                rule.id.clone(),
                                name.to_owned(),
                                v.value.clone(),
                            )));
                        }
                    }
                });
        }
        Param::Int { min, max, .. } => {
            let mut v = current.as_i64().unwrap_or(*min);
            if ui
                .add(egui::DragValue::new(&mut v).range(*min..=*max))
                .changed()
            {
                acts.push(Act::Profile(ProfileAct::SetParam(
                    rule.id.clone(),
                    name.to_owned(),
                    Value::from(v),
                )));
            }
        }
        Param::List { .. } => {
            let pairs = rule.actions.iter().any(|a| {
                matches!(a, Action::RegList { param, names: ListNames::Pairs, .. } if param == name)
            });
            let mut text = current
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            let hint = if pairs {
                tr("One name=value per line")
            } else {
                tr("One item per line")
            };
            if ui
                .add(
                    egui::TextEdit::multiline(&mut text)
                        .id_salt(format!("{}.{name}", rule.id))
                        .desired_rows(6)
                        .desired_width((ui.available_width() - 24.0).clamp(360.0, 760.0))
                        .hint_text(hint),
                )
                .changed()
            {
                let items: Vec<Value> = if text.is_empty() {
                    Vec::new()
                } else {
                    text.split('\n')
                        .map(|l| Value::from(l.trim_end_matches('\r')))
                        .collect()
                };
                acts.push(Act::Profile(ProfileAct::SetParam(
                    rule.id.clone(),
                    name.to_owned(),
                    Value::Array(items),
                )));
            }
        }
        Param::Text { .. } => {
            let mut s = current.as_str().unwrap_or("").to_owned();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut s)
                        .id_salt(format!("{}.{name}", rule.id))
                        .desired_width((ui.available_width() - 24.0).clamp(240.0, 520.0)),
                )
                .changed()
            {
                acts.push(Act::Profile(ProfileAct::SetParam(
                    rule.id.clone(),
                    name.to_owned(),
                    Value::from(s),
                )));
            }
        }
    }
    ui.label(style::muted(
        ui,
        trf("default: {value}", &[("value", &param.default_value())]),
        11.0,
    ));
}
